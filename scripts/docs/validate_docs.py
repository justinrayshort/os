#!/usr/bin/env python3
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


FRONTMATTER_DELIM = "---"
SCHEME_RE = re.compile(r"^[a-zA-Z][a-zA-Z0-9+.-]*:")
MARKDOWN_LINK_RE = re.compile(r"(?<!!)\[[^\]]+\]\(([^)]+)\)")
HEADER_RE = re.compile(r"^(#{1,6})\s+(.*)$")
FENCE_RE = re.compile(r"^```(?P<lang>[A-Za-z0-9_-]*)\s*$")


@dataclass(frozen=True)
class Problem:
    check: str
    path: str
    message: str
    line: int | None = None


@dataclass(frozen=True)
class LinkRef:
    target: str
    line: int


@dataclass
class DocRecord:
    path: Path
    rel_path: str
    text: str
    frontmatter: dict
    body: str
    headings: list[str]
    anchors: set[str]
    links: list[LinkRef]


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def docs_root() -> Path:
    return repo_root() / "docs"


def load_contracts() -> dict:
    path = repo_root() / "tools" / "docs" / "doc_contracts.json"
    return json.loads(path.read_text(encoding="utf-8"))


def strip_quotes(value: str) -> str:
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in ("'", '"'):
        return value[1:-1]
    return value


def parse_scalar(value: str):
    value = value.strip()
    if value == "":
        return ""
    if value.lower() == "true":
        return True
    if value.lower() == "false":
        return False
    return strip_quotes(value)


def parse_inline_list(value: str) -> list:
    inner = value[1:-1].strip()
    if not inner:
        return []
    items: list[str] = []
    buf = []
    quote: str | None = None
    for ch in inner:
        if quote:
            if ch == quote:
                quote = None
            buf.append(ch)
            continue
        if ch in ("'", '"'):
            quote = ch
            buf.append(ch)
            continue
        if ch == ",":
            items.append("".join(buf).strip())
            buf = []
            continue
        buf.append(ch)
    if buf:
        items.append("".join(buf).strip())
    return [parse_scalar(item) for item in items if item.strip()]


def parse_yamlish_frontmatter(raw: str) -> tuple[dict, list[str]]:
    data: dict = {}
    errors: list[str] = []
    current_list_key: str | None = None

    for idx, raw_line in enumerate(raw.splitlines(), start=1):
        line = raw_line.rstrip()
        if not line.strip():
            continue
        if re.match(r"^\s*#.*$", line):
            continue

        list_match = re.match(r"^\s*-\s+(.*)$", line)
        if list_match:
            if current_list_key is None or not isinstance(data.get(current_list_key), list):
                errors.append(f"frontmatter line {idx}: list item without list key")
                continue
            data[current_list_key].append(parse_scalar(list_match.group(1)))
            continue

        key_match = re.match(r"^([A-Za-z0-9_-]+):\s*(.*)$", line)
        if not key_match:
            errors.append(f"frontmatter line {idx}: unsupported syntax `{line}`")
            current_list_key = None
            continue

        key = key_match.group(1)
        value = key_match.group(2)
        if value == "":
            data[key] = []
            current_list_key = key
        elif value.startswith("[") and value.endswith("]"):
            data[key] = parse_inline_list(value)
            current_list_key = None
        else:
            data[key] = parse_scalar(value)
            current_list_key = None

    return data, errors


def split_frontmatter(text: str) -> tuple[dict, str, list[str]]:
    if not text.startswith(FRONTMATTER_DELIM):
        return {}, text, ["missing frontmatter start delimiter"]

    lines = text.splitlines()
    if not lines or lines[0].strip() != FRONTMATTER_DELIM:
        return {}, text, ["invalid frontmatter start delimiter"]

    end_idx = None
    for i in range(1, len(lines)):
        if lines[i].strip() == FRONTMATTER_DELIM:
            end_idx = i
            break
    if end_idx is None:
        return {}, text, ["missing frontmatter end delimiter"]

    raw_frontmatter = "\n".join(lines[1:end_idx])
    body = "\n".join(lines[end_idx + 1 :])
    fm, fm_errors = parse_yamlish_frontmatter(raw_frontmatter)
    return fm, body, fm_errors


def extract_headings_and_anchors(body: str) -> tuple[list[str], set[str]]:
    headings: list[str] = []
    anchors: set[str] = set()
    counts: dict[str, int] = defaultdict(int)
    in_fence = False

    for line in body.splitlines():
        if FENCE_RE.match(line):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        m = HEADER_RE.match(line)
        if not m:
            continue
        text = m.group(2).strip()
        headings.append(text)

        base = slugify_heading(text)
        if not base:
            continue
        count = counts[base]
        counts[base] += 1
        anchor = base if count == 0 else f"{base}-{count}"
        anchors.add(anchor)

    return headings, anchors


def slugify_heading(text: str) -> str:
    value = text.strip().lower()
    value = value.replace("`", "")
    value = re.sub(r"[^\w\s-]", "", value)
    value = re.sub(r"\s+", "-", value)
    value = re.sub(r"-{2,}", "-", value)
    return value.strip("-")


def extract_links(body: str) -> list[LinkRef]:
    links: list[LinkRef] = []
    in_fence = False
    for line_no, line in enumerate(body.splitlines(), start=1):
        if FENCE_RE.match(line):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        for match in MARKDOWN_LINK_RE.finditer(line):
            target = match.group(1).strip()
            # Drop optional title part in markdown link syntax if present.
            if " " in target and not target.startswith("<"):
                target = target.split(" ", 1)[0]
            if target.startswith("<") and target.endswith(">"):
                target = target[1:-1]
            links.append(LinkRef(target=target, line=line_no))
    return links


def collect_docs() -> tuple[list[DocRecord], list[Problem]]:
    root = docs_root()
    problems: list[Problem] = []
    records: list[DocRecord] = []
    for path in sorted(root.rglob("*.md")):
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        fm, body, fm_errors = split_frontmatter(text)
        rel = path.relative_to(repo_root()).as_posix()
        for err in fm_errors:
            problems.append(Problem("frontmatter", rel, err))
        headings, anchors = extract_headings_and_anchors(body)
        links = extract_links(body)
        records.append(
            DocRecord(
                path=path,
                rel_path=rel,
                text=text,
                frontmatter=fm,
                body=body,
                headings=headings,
                anchors=anchors,
                links=links,
            )
        )
    return records, problems


def validate_structure(contracts: dict) -> list[Problem]:
    problems: list[Problem] = []
    root = docs_root()
    for dirname in contracts.get("required_docs_directories", []):
        path = root / dirname
        if not path.exists():
            problems.append(Problem("structure", f"docs/{dirname}", "required directory is missing"))
    return problems


def parse_review_date(value: str) -> dt.date | None:
    try:
        return dt.date.fromisoformat(value)
    except Exception:
        return None


def current_date() -> dt.date:
    override = os.environ.get("DOCS_TODAY")
    if override:
        return dt.date.fromisoformat(override)
    return dt.date.today()


def validate_frontmatter(records: list[DocRecord], contracts: dict) -> list[Problem]:
    problems: list[Problem] = []
    required_fields = contracts.get("required_frontmatter", [])
    allowed_categories = set(contracts.get("allowed_categories", []))
    allowed_statuses = set(contracts.get("allowed_statuses", []))
    allowed_owners = set(contracts.get("allowed_owners", []))
    folder_map = contracts.get("diataxis_category_by_folder", {})
    root_allowed = set(contracts.get("root_docs_allowed_categories", []))
    stale_days = int(os.environ.get("DOCS_STALE_REVIEW_DAYS", contracts.get("stale_review_days", 180)))
    today = current_date()

    for record in records:
        fm = record.frontmatter
        for field in required_fields:
            if field not in fm:
                problems.append(Problem("frontmatter", record.rel_path, f"missing required field `{field}`"))

        if not fm:
            continue

        title = fm.get("title")
        if not isinstance(title, str) or not title.strip():
            problems.append(Problem("frontmatter", record.rel_path, "`title` must be a non-empty string"))

        category = fm.get("category")
        if not isinstance(category, str) or category not in allowed_categories:
            problems.append(Problem("frontmatter", record.rel_path, f"`category` must be one of {sorted(allowed_categories)}"))

        owner = fm.get("owner")
        if not isinstance(owner, str) or owner not in allowed_owners:
            problems.append(Problem("frontmatter", record.rel_path, f"`owner` must be one of {sorted(allowed_owners)}"))

        status = fm.get("status")
        if not isinstance(status, str) or status not in allowed_statuses:
            problems.append(Problem("frontmatter", record.rel_path, f"`status` must be one of {sorted(allowed_statuses)}"))
        if status == "superseded" and not isinstance(fm.get("superseded_by"), str):
            problems.append(Problem("frontmatter", record.rel_path, "`superseded` docs must declare `superseded_by`"))

        reviewed = fm.get("last_reviewed")
        if not isinstance(reviewed, str):
            problems.append(Problem("frontmatter", record.rel_path, "`last_reviewed` must be an ISO date string"))
        else:
            review_date = parse_review_date(reviewed)
            if review_date is None:
                problems.append(Problem("frontmatter", record.rel_path, "`last_reviewed` is not a valid ISO date"))
            else:
                age_days = (today - review_date).days
                if age_days > stale_days:
                    problems.append(
                        Problem(
                            "frontmatter",
                            record.rel_path,
                            f"`last_reviewed` is stale ({age_days} days > {stale_days})",
                        )
                    )

        for list_field in ("audience", "invariants"):
            value = fm.get(list_field)
            if not isinstance(value, list) or not value:
                problems.append(Problem("frontmatter", record.rel_path, f"`{list_field}` must be a non-empty list"))
            elif not all(isinstance(v, str) and str(v).strip() for v in value):
                problems.append(
                    Problem("frontmatter", record.rel_path, f"`{list_field}` must contain non-empty strings")
                )

        rel_doc = record.path.relative_to(docs_root())
        if len(rel_doc.parts) == 1:
            if isinstance(category, str) and category not in root_allowed:
                problems.append(
                    Problem(
                        "diataxis",
                        record.rel_path,
                        f"root docs page category `{category}` not allowed; expected one of {sorted(root_allowed)}",
                    )
                )
        else:
            folder = rel_doc.parts[0]
            expected = folder_map.get(folder)
            if expected and category != expected:
                problems.append(
                    Problem("diataxis", record.rel_path, f"category `{category}` does not match folder `{folder}` -> `{expected}`")
                )

            if folder == "adr" and not re.match(r"ADR-\d{4}[-_a-zA-Z0-9]*\.md$", record.path.name):
                problems.append(Problem("adr", record.rel_path, "ADR filename must match ADR-0000-name.md"))

    return problems


def validate_sop_headings(records: list[DocRecord], contracts: dict) -> list[Problem]:
    required = [normalize_heading(h) for h in contracts.get("sop_required_headings", [])]
    problems: list[Problem] = []

    for record in records:
        rel_doc = record.path.relative_to(docs_root())
        if not rel_doc.parts or rel_doc.parts[0] != "sop":
            continue
        headings = [normalize_heading(h) for h in record.headings]
        pos = 0
        for req in required:
            try:
                found_at = headings.index(req, pos)
            except ValueError:
                problems.append(Problem("sop", record.rel_path, f"missing or out-of-order heading `{req}`"))
                break
            pos = found_at + 1
    return problems


def normalize_heading(value: str) -> str:
    return re.sub(r"\s+", " ", value.strip())


def resolve_markdown_link(record: DocRecord, target: str) -> tuple[Path | None, str | None, str | None]:
    # Returns (resolved_path, anchor, skip_reason)
    if not target or target.startswith("?"):
        return None, None, "query-only link"
    if SCHEME_RE.match(target):
        return None, None, "external link"
    if target.startswith("//"):
        return None, None, "network-path link"

    path_part, _, anchor = target.partition("#")
    if path_part and "?" in path_part:
        # Route-style links like /?open=notes:foo are not docs file links.
        return None, anchor or None, "route/query link"

    if path_part.startswith("/") and not path_part.startswith("/docs/"):
        return None, anchor or None, "site route"

    if not path_part:
        return record.path, anchor or None, None

    if path_part.startswith("/docs/"):
        resolved = repo_root() / path_part.lstrip("/")
    elif path_part.startswith("/"):
        resolved = repo_root() / path_part.lstrip("/")
    else:
        resolved = (record.path.parent / path_part).resolve()

    if resolved.exists():
        return resolved, anchor or None, None
    if resolved.suffix == "":
        md_candidate = resolved.with_suffix(".md")
        if md_candidate.exists():
            return md_candidate, anchor or None, None
        index_candidate = resolved / "index.md"
        if index_candidate.exists():
            return index_candidate, anchor or None, None

    return resolved, anchor or None, None


def validate_links(records: list[DocRecord]) -> list[Problem]:
    problems: list[Problem] = []
    record_by_path = {r.path.resolve(): r for r in records}
    heading_cache = {r.path.resolve(): r.anchors for r in records}

    for record in records:
        for link in record.links:
            resolved, anchor, skip_reason = resolve_markdown_link(record, link.target)
            if skip_reason is not None:
                continue
            if resolved is None:
                continue
            resolved_key = resolved.resolve()
            if not resolved_key.exists():
                problems.append(
                    Problem(
                        "links",
                        record.rel_path,
                        f"broken link target `{link.target}` (resolved `{resolved}`)",
                        line=link.line,
                    )
                )
                continue
            if anchor and resolved_key.suffix.lower() == ".md":
                anchors = heading_cache.get(resolved_key)
                if anchors is None:
                    # Markdown file outside docs/ is allowed if it exists, but anchor validation is skipped.
                    continue
                if anchor not in anchors:
                    problems.append(
                        Problem(
                            "links",
                            record.rel_path,
                            f"missing anchor `#{anchor}` in `{resolved_key.relative_to(repo_root()).as_posix()}`",
                            line=link.line,
                        )
                    )

    return problems


def extract_fenced_mermaid_blocks(record: DocRecord) -> tuple[list[tuple[int, str]], list[Problem]]:
    blocks: list[tuple[int, str]] = []
    problems: list[Problem] = []
    in_fence = False
    fence_lang = ""
    fence_start_line = 0
    buffer: list[str] = []

    for line_no, line in enumerate(record.body.splitlines(), start=1):
        fence = FENCE_RE.match(line)
        if fence:
            if not in_fence:
                in_fence = True
                fence_lang = fence.group("lang").strip().lower()
                fence_start_line = line_no
                buffer = []
            else:
                if fence_lang == "mermaid":
                    source = "\n".join(buffer).strip()
                    if not source:
                        problems.append(
                            Problem("mermaid", record.rel_path, "empty mermaid block", line=fence_start_line)
                        )
                    else:
                        blocks.append((fence_start_line, source))
                in_fence = False
                fence_lang = ""
                buffer = []
            continue

        if in_fence:
            buffer.append(line)

    if in_fence:
        problems.append(Problem("mermaid", record.rel_path, "unclosed code fence", line=fence_start_line))

    return blocks, problems


def render_mermaid_sources(sources: list[tuple[str, int | None, str]], require_renderer: bool) -> list[Problem]:
    problems: list[Problem] = []
    mmdc = shutil.which("mmdc")
    if not mmdc:
        if require_renderer and sources:
            problems.append(Problem("mermaid", "mmdc", "mermaid-cli (`mmdc`) is required but not installed"))
        return problems

    for label, line, source in sources:
        with tempfile.TemporaryDirectory(prefix="mmdc-") as td:
            td_path = Path(td)
            in_file = td_path / "diagram.mmd"
            out_file = td_path / "diagram.svg"
            in_file.write_text(source + "\n", encoding="utf-8")
            result = subprocess.run(
                [mmdc, "-i", str(in_file), "-o", str(out_file), "-q"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                check=False,
            )
            if result.returncode != 0:
                msg = (result.stderr or result.stdout or "mmdc failed").strip().splitlines()[-1]
                problems.append(Problem("mermaid", label, f"render validation failed: {msg}", line=line))
    return problems


def validate_mermaid(records: list[DocRecord], require_renderer: bool = False) -> tuple[list[Problem], int]:
    problems: list[Problem] = []
    sources: list[tuple[str, int | None, str]] = []
    count = 0

    for record in records:
        blocks, block_problems = extract_fenced_mermaid_blocks(record)
        problems.extend(block_problems)
        for line, source in blocks:
            count += 1
            sources.append((record.rel_path, line, source))

    for path in sorted((docs_root() / "assets").rglob("*.mmd")):
        source = path.read_text(encoding="utf-8").strip()
        rel = path.relative_to(repo_root()).as_posix()
        if not source:
            problems.append(Problem("mermaid", rel, "empty .mmd diagram file"))
            continue
        count += 1
        sources.append((rel, None, source))

    problems.extend(render_mermaid_sources(sources, require_renderer=require_renderer))
    return problems, count


def basic_openapi_sanity_check(path: Path) -> str | None:
    if path.suffix.lower() == ".json":
        try:
            with path.open("r", encoding="utf-8") as fh:
                data = json.load(fh)
            if not isinstance(data, dict) or ("openapi" not in data and "swagger" not in data):
                return "missing `openapi` or `swagger` top-level key"
            return None
        except Exception as exc:
            return f"invalid JSON: {exc}"

    text = path.read_text(encoding="utf-8")
    if re.search(r"^\s*(openapi|swagger)\s*:\s*", text, flags=re.MULTILINE):
        return None
    return "missing `openapi:` or `swagger:` declaration"


def validate_openapi(require_validator: bool = False) -> tuple[list[Problem], int]:
    problems: list[Problem] = []
    specs = [
        p
        for p in sorted((docs_root() / "reference" / "openapi").rglob("*"))
        if p.is_file() and p.suffix.lower() in {".yaml", ".yml", ".json"}
    ]
    if not specs:
        return problems, 0

    swagger_cli = shutil.which("swagger-cli")
    if require_validator and not swagger_cli:
        problems.append(
            Problem("openapi", "swagger-cli", "OpenAPI validator (`swagger-cli`) is required but not installed")
        )
        return problems, len(specs)

    for spec in specs:
        rel = spec.relative_to(repo_root()).as_posix()
        if swagger_cli:
            result = subprocess.run(
                [swagger_cli, "validate", str(spec)],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                check=False,
            )
            if result.returncode != 0:
                msg = (result.stderr or result.stdout or "swagger-cli validate failed").strip().splitlines()[-1]
                problems.append(Problem("openapi", rel, msg))
        else:
            err = basic_openapi_sanity_check(spec)
            if err:
                problems.append(Problem("openapi", rel, err))
    return problems, len(specs)


def print_problems(problems: list[Problem]) -> None:
    for p in sorted(problems, key=lambda x: (x.check, x.path, x.line or 0, x.message)):
        loc = f"{p.path}:{p.line}" if p.line else p.path
        print(f"[{p.check}] {loc} - {p.message}")


def fail_if_problems(problems: list[Problem]) -> int:
    if problems:
        print_problems(problems)
        print(f"\nFAILED: {len(problems)} issue(s)")
        return 1
    print("OK")
    return 0


def frontmatter_freshness_metrics(records: list[DocRecord], contracts: dict) -> tuple[int, int, list[str]]:
    stale_days = int(os.environ.get("DOCS_STALE_REVIEW_DAYS", contracts.get("stale_review_days", 180)))
    today = current_date()
    fresh = 0
    total = 0
    stale_docs: list[str] = []
    for record in records:
        reviewed = record.frontmatter.get("last_reviewed")
        if not isinstance(reviewed, str):
            continue
        parsed = parse_review_date(reviewed)
        if parsed is None:
            continue
        total += 1
        if (today - parsed).days <= stale_days:
            fresh += 1
        else:
            stale_docs.append(record.rel_path)
    return fresh, total, stale_docs


def counts_by_category(records: list[DocRecord]) -> dict[str, int]:
    counter: Counter[str] = Counter()
    for record in records:
        category = record.frontmatter.get("category", "unknown")
        if not isinstance(category, str):
            category = "unknown"
        counter[category] += 1
    return dict(sorted(counter.items()))


def run_all(records: list[DocRecord], parse_problems: list[Problem], contracts: dict, args) -> int:
    problems = list(parse_problems)
    problems.extend(validate_structure(contracts))
    problems.extend(validate_frontmatter(records, contracts))
    problems.extend(validate_sop_headings(records, contracts))
    problems.extend(validate_links(records))
    mermaid_problems, _ = validate_mermaid(records, require_renderer=getattr(args, "require_renderer", False))
    openapi_problems, _ = validate_openapi(require_validator=getattr(args, "require_openapi_validator", False))
    problems.extend(mermaid_problems)
    problems.extend(openapi_problems)
    return fail_if_problems(problems)


def write_audit_report(records: list[DocRecord], parse_problems: list[Problem], contracts: dict, output: Path) -> int:
    structure_problems = validate_structure(contracts)
    frontmatter_problems = validate_frontmatter(records, contracts)
    sop_problems = validate_sop_headings(records, contracts)
    link_problems = validate_links(records)
    mermaid_problems, mermaid_count = validate_mermaid(records, require_renderer=False)
    openapi_problems, openapi_count = validate_openapi(require_validator=False)

    fresh, total_reviewed, stale_docs = frontmatter_freshness_metrics(records, contracts)
    fresh_percent = round((fresh / total_reviewed * 100.0), 2) if total_reviewed else 0.0

    report = {
        "generated_at": dt.datetime.now(dt.UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "document_count": len(records),
        "markdown_count": len(records),
        "counts_by_category": counts_by_category(records),
        "fresh_review_percent": fresh_percent,
        "stale_documents": stale_docs,
        "broken_internal_links": len([p for p in link_problems if p.check == "links"]),
        "mermaid_block_count": mermaid_count,
        "openapi_spec_count": openapi_count,
        "validation_issue_counts": {
            "parse": len(parse_problems),
            "structure": len(structure_problems),
            "frontmatter": len(frontmatter_problems),
            "sop": len(sop_problems),
            "links": len(link_problems),
            "mermaid": len(mermaid_problems),
            "openapi": len(openapi_problems),
        },
    }

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"Wrote audit report: {output}")

    all_problems = (
        parse_problems
        + structure_problems
        + frontmatter_problems
        + sop_problems
        + link_problems
        + mermaid_problems
        + openapi_problems
    )
    if all_problems:
        print_problems(all_problems)
        print(f"\nFAILED: {len(all_problems)} issue(s)")
        return 1
    print("OK")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Documentation contract and integrity validator")
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("structure")
    sub.add_parser("frontmatter")
    sub.add_parser("sop")
    sub.add_parser("links")

    mermaid = sub.add_parser("mermaid")
    mermaid.add_argument("--require-renderer", action="store_true", help="Fail if `mmdc` is unavailable")

    openapi = sub.add_parser("openapi")
    openapi.add_argument("--require-validator", action="store_true", help="Fail if `swagger-cli` is unavailable")

    all_cmd = sub.add_parser("all")
    all_cmd.add_argument("--require-renderer", action="store_true", help="Require `mmdc` for Mermaid render validation")
    all_cmd.add_argument(
        "--require-openapi-validator",
        action="store_true",
        help="Require `swagger-cli` for OpenAPI validation",
    )

    audit = sub.add_parser("audit-report")
    audit.add_argument("--output", required=True, help="Write JSON audit report to this path")

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    contracts = load_contracts()
    records, parse_problems = collect_docs()

    if args.command == "structure":
        return fail_if_problems(validate_structure(contracts))
    if args.command == "frontmatter":
        return fail_if_problems(parse_problems + validate_frontmatter(records, contracts))
    if args.command == "sop":
        return fail_if_problems(validate_sop_headings(records, contracts))
    if args.command == "links":
        return fail_if_problems(validate_links(records))
    if args.command == "mermaid":
        problems, count = validate_mermaid(records, require_renderer=args.require_renderer)
        print(f"Mermaid sources checked: {count}")
        return fail_if_problems(problems)
    if args.command == "openapi":
        problems, count = validate_openapi(require_validator=args.require_validator)
        print(f"OpenAPI specs checked: {count}")
        return fail_if_problems(problems)
    if args.command == "all":
        return run_all(records, parse_problems, contracts, args)
    if args.command == "audit-report":
        return write_audit_report(records, parse_problems, contracts, Path(args.output))

    parser.error(f"unsupported command: {args.command}")
    return 2


if __name__ == "__main__":
    sys.exit(main())
