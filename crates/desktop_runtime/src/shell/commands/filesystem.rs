use std::rc::Rc;

use desktop_app_contract::AppCommandRegistration;
use platform_host::{ExplorerEntryKind, ExplorerFsService};
use platform_host_web::explorer_fs_service;
use system_shell_contract::{CommandArgSpec, CommandDataShape, CommandNotice, CommandNoticeLevel, CommandOutputShape};

pub(super) fn registrations() -> Vec<AppCommandRegistration> {
    vec![pwd_registration(), cd_registration(), ls_registration()]
}

fn pwd_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: super::super::root_descriptor(
            "pwd",
            &[],
            "Print the logical filesystem cwd.",
            "pwd",
            Vec::new(),
            Vec::new(),
            system_shell_contract::CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Scalar),
        ),
        completion: None,
        handler: Rc::new(|context| {
            Box::pin(async move {
                Ok(system_shell_contract::CommandResult {
                    output: super::super::string_data(context.cwd),
                    display: system_shell_contract::DisplayPreference::Value,
                    notices: Vec::new(),
                    cwd: None,
                    exit: system_shell_contract::ShellExit::success(),
                })
            })
        }),
    }
}

fn cd_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: super::super::root_descriptor(
            "cd",
            &[],
            "Change the logical filesystem cwd.",
            "cd <path>",
            vec![CommandArgSpec {
                name: "path".to_string(),
                summary: "Target directory path.".to_string(),
                required: true,
                repeatable: false,
            }],
            Vec::new(),
            system_shell_contract::CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Empty),
        ),
        completion: Some(Rc::new(|request| {
            let raw = request.argv.get(1).cloned().unwrap_or_default();
            super::super::path_completion_items(&request.cwd, &raw, true)
        })),
        handler: Rc::new(|context| {
            Box::pin(async move {
                let target = context
                    .args
                    .first()
                    .ok_or_else(|| super::super::usage_error("usage: cd <path>"))?;
                let resolved = super::super::normalize_session_path(&context.cwd, target);
                let metadata = explorer_fs_service()
                    .stat(&resolved)
                    .await
                    .map_err(super::super::unavailable)?;
                if metadata.kind != ExplorerEntryKind::Directory {
                    return Err(super::super::usage_error(format!(
                        "not a directory: `{resolved}`"
                    )));
                }
                let mut result = super::super::info_result(format!("cwd = {resolved}"));
                result.cwd = Some(resolved);
                Ok(result)
            })
        }),
    }
}

fn ls_registration() -> AppCommandRegistration {
    AppCommandRegistration {
        descriptor: super::super::root_descriptor(
            "ls",
            &[],
            "List a directory using the active explorer backend.",
            "ls [path]",
            vec![CommandArgSpec {
                name: "path".to_string(),
                summary: "Optional target directory.".to_string(),
                required: false,
                repeatable: false,
            }],
            Vec::new(),
            system_shell_contract::CommandInputShape::none(),
            CommandOutputShape::new(CommandDataShape::Table),
        ),
        completion: Some(Rc::new(|request| {
            let raw = request.argv.get(1).cloned().unwrap_or_default();
            super::super::path_completion_items(&request.cwd, &raw, false)
        })),
        handler: Rc::new(|context| {
            Box::pin(async move {
                let target = context
                    .args
                    .first()
                    .map(|path| super::super::normalize_session_path(&context.cwd, path))
                    .unwrap_or_else(|| context.cwd.clone());
                let listing = explorer_fs_service()
                    .list_dir(&target)
                    .await
                    .map_err(super::super::unavailable)?;
                Ok(system_shell_contract::CommandResult {
                    output: super::super::table_data(
                        vec![
                            "name".to_string(),
                            "kind".to_string(),
                            "path".to_string(),
                            "size".to_string(),
                            "modified_at_unix_ms".to_string(),
                        ],
                        listing.entries.iter().map(super::super::explorer_row).collect(),
                        Some(system_shell_contract::CommandPath::new("ls")),
                    ),
                    display: system_shell_contract::DisplayPreference::Table,
                    notices: vec![CommandNotice {
                        level: CommandNoticeLevel::Info,
                        message: format!("listed {}", listing.cwd),
                    }],
                    cwd: None,
                    exit: system_shell_contract::ShellExit::success(),
                })
            })
        }),
    }
}
