async page => {
  page.setDefaultTimeout(10000);
  let phase = 'init';
  const assert = (cond, msg) => { if (!cond) throw new Error(msg); };
  const dialogByName = name => page.getByRole('dialog', { name }).last();
  const readDesktopRow = async () => page.evaluate(async () => {
    const open = indexedDB.open('retrodesk_os', 1);
    const db = await new Promise((res, rej) => {
      open.onsuccess = () => res(open.result);
      open.onerror = () => rej(open.error);
    });
    const tx = db.transaction('app_state', 'readonly');
    const store = tx.objectStore('app_state');
    const row = await new Promise((res, rej) => {
      const r = store.get('system.desktop');
      r.onsuccess = () => res(r.result);
      r.onerror = () => rej(r.error);
    });
    return row || null;
  });

  try {
    phase = 'goto';
    await page.goto('http://127.0.0.1:8081/');
    await page.waitForLoadState('networkidle');

    phase = 'open-explorer';
    await page.getByRole('button', { name: 'DIR Explorer' }).click();
    const explorer = dialogByName('Explorer');
    await explorer.waitFor();
    await explorer.getByText('Backend', { exact: true }).waitFor();
    await page.waitForFunction(() => Array.from(document.querySelectorAll('.app-statusbar span')).some(el => (el.textContent || '').includes('Backend: IndexedDbVirtual')));

    phase = 'explorer-enter-documents';
    await explorer.getByRole('row', { name: /Documents Folder/ }).evaluate(el => el.dispatchEvent(new MouseEvent('dblclick', { bubbles: true })));
    await explorer.getByText('Path: /Documents').waitFor();

    phase = 'explorer-create-save-file';
    await explorer.getByRole('textbox', { name: 'New item name' }).fill('smoke.txt');
    await explorer.getByRole('button', { name: 'New File' }).click({ force: true });
    const explorerEditor = explorer.getByRole('textbox', { name: 'Explorer text file editor' });
    await explorerEditor.waitFor();
    const explorerText = 'smoke test file\nhello from playwright\n';
    await explorerEditor.fill(explorerText);
    await explorer.getByRole('button', { name: 'Save' }).click({ force: true });
    await page.waitForFunction(() => Array.from(document.querySelectorAll('.explorer-editor .pane-path')).some(el => (el.textContent || '').trim() === 'Saved'));

    phase = 'open-notepad';
    await page.getByRole('button', { name: /Pinned Notepad/ }).click({ force: true });
    let notepad = dialogByName('Notepad');
    try {
      await notepad.waitFor({ timeout: 3000 });
    } catch {
      // Some CLI click paths do not dispatch a full click sequence on taskbar buttons; retry with direct DOM click.
      await page.getByRole('button', { name: /Pinned Notepad/ }).evaluate(el => el.click());
      await notepad.waitFor({ timeout: 5000 });
    }

    phase = 'edit-notepad';
    const noteEditor = notepad.getByRole('textbox', { name: 'Notepad document editor' });
    await noteEditor.waitFor();
    const noteMarker = '\n[smoke] persisted note marker';
    const currentNote = await noteEditor.inputValue();
    if (!currentNote.includes('[smoke] persisted note marker')) {
      await noteEditor.fill(currentNote + noteMarker);
    }

    phase = 'open-calculator';
    await page.getByRole('button', { name: /Pinned Calculator/ }).click({ force: true });
    let calc = dialogByName('Calculator');
    try {
      await calc.waitFor({ timeout: 3000 });
    } catch {
      await page.getByRole('button', { name: /Pinned Calculator/ }).evaluate(el => el.click());
      await calc.waitFor({ timeout: 5000 });
    }

    phase = 'calculate';
    await calc.getByRole('button', { name: /^C$/ }).click();
    await calc.getByRole('button', { name: /^2$/ }).click();
    await calc.getByRole('button', { name: /^\+$/ }).click();
    await calc.getByRole('button', { name: /^3$/ }).click();
    await calc.getByRole('button', { name: /^=$/ }).click();
    const calcBeforeReload = (await calc.locator('.calc-display').textContent())?.trim() || '';
    assert(calcBeforeReload === '5', `Calculator pre-reload display expected 5, got ${calcBeforeReload}`);

    phase = 'wait-durable-layout-save';
    let beforeReloadRow = null;
    for (let i = 0; i < 50; i += 1) {
      beforeReloadRow = await readDesktopRow();
      const windowCount = ((beforeReloadRow && beforeReloadRow.payload && beforeReloadRow.payload.windows) || []).length;
      if (windowCount >= 3) break;
      await page.waitForTimeout(100);
    }
    const beforeReloadCount = ((beforeReloadRow && beforeReloadRow.payload && beforeReloadRow.payload.windows) || []).length;
    assert(beforeReloadCount >= 3, `Desktop durable snapshot did not reach 3 windows before reload (got ${beforeReloadCount})`);

    phase = 'reload';
    await page.reload();
    await page.waitForLoadState('networkidle');

    phase = 'rehydrate-windows';
    const explorer2 = dialogByName('Explorer');
    const notepad2 = dialogByName('Notepad');
    const calc2 = dialogByName('Calculator');
    await explorer2.waitFor();
    await notepad2.waitFor();
    await calc2.waitFor();

    phase = 'verify-hydrated-state';
    const explorerRestored = await explorer2.getByRole('textbox', { name: 'Explorer text file editor' }).inputValue();
    const noteRestored = await notepad2.getByRole('textbox', { name: 'Notepad document editor' }).inputValue();
    const calcAfterReload = (await calc2.locator('.calc-display').textContent())?.trim() || '';
    const calcTapeItems = await calc2.locator('.calc-tape-item').count();
    const afterReloadRow = await readDesktopRow();
    assert(explorerRestored.includes('hello from playwright'), 'Explorer editor content did not hydrate');
    assert(noteRestored.includes('[smoke] persisted note marker'), 'Notepad content did not hydrate');
    assert(calcAfterReload === '5', `Calculator display did not hydrate, got ${calcAfterReload}`);
    assert(calcTapeItems >= 1, 'Calculator tape did not hydrate');

    phase = 'screenshot';
    await page.screenshot({ path: 'output/playwright/smoke-after-reload.png', fullPage: true });

    return {
      explorerLen: explorerRestored.length,
      noteLen: noteRestored.length,
      calcBeforeReload,
      calcAfterReload,
      calcTapeItems,
      beforeReloadDesktopWindows: beforeReloadCount,
      afterReloadDesktopWindows: ((afterReloadRow && afterReloadRow.payload && afterReloadRow.payload.windows) || []).length,
      beforeReloadDesktopTs: beforeReloadRow && beforeReloadRow.updated_at_unix_ms,
      afterReloadDesktopTs: afterReloadRow && afterReloadRow.updated_at_unix_ms,
      phase: 'done'
    };
  } catch (err) {
    let desktopDiag = '';
    let dialogDiag = '';
    try {
      const row = await readDesktopRow();
      const windows = ((row && row.payload && row.payload.windows) || []).map(w => w.title || w.app_id);
      desktopDiag = ` | desktopRow.ts=${row && row.updated_at_unix_ms} desktopRow.windows=${windows.length} [${windows.join(', ')}]`;
    } catch (diagErr) {
      desktopDiag = ` | desktopRow.readError=${diagErr && diagErr.message ? diagErr.message : String(diagErr)}`;
    }
    try {
      const dialogs = await page.evaluate(() =>
        Array.from(document.querySelectorAll('[role="dialog"]'))
          .map(el => (el.getAttribute('aria-label') || el.getAttribute('aria-labelledby') || el.textContent || '').trim())
          .slice(0, 10)
      );
      dialogDiag = ` | domDialogs=${dialogs.length}`;
    } catch (diagErr) {
      dialogDiag = ` | domDialogs.readError=${diagErr && diagErr.message ? diagErr.message : String(diagErr)}`;
    }
    throw new Error(`[phase=${phase}] ${err && err.message ? err.message : String(err)}${desktopDiag}${dialogDiag}`);
  }
}
