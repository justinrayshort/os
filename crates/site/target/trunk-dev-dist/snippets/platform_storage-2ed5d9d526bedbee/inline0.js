
const DB_NAME = 'retrodesk_os';
const DB_VERSION = 1;
const APP_STATE_STORE = 'app_state';
const VFS_STORE = 'vfs_nodes';
const FS_CONFIG_STORE = 'fs_config';

function fail(message) {
  throw new Error(message);
}

function idbSupported() {
  return typeof indexedDB !== 'undefined';
}

function requestToPromise(req) {
  return new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error || new Error('IndexedDB request failed'));
  });
}

function txDone(tx) {
  return new Promise((resolve, reject) => {
    tx.oncomplete = () => resolve();
    tx.onabort = () => reject(tx.error || new Error('IndexedDB transaction aborted'));
    tx.onerror = () => reject(tx.error || new Error('IndexedDB transaction error'));
  });
}

async function openDb() {
  if (!idbSupported()) {
    fail('IndexedDB is unavailable in this browser context');
  }
  return await new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(APP_STATE_STORE)) {
        db.createObjectStore(APP_STATE_STORE, { keyPath: 'namespace' });
      }
      if (!db.objectStoreNames.contains(VFS_STORE)) {
        const store = db.createObjectStore(VFS_STORE, { keyPath: 'path' });
        store.createIndex('by_parent', 'parent', { unique: false });
      }
      if (!db.objectStoreNames.contains(FS_CONFIG_STORE)) {
        db.createObjectStore(FS_CONFIG_STORE, { keyPath: 'key' });
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error || new Error('Failed to open IndexedDB'));
  });
}

async function withStore(storeName, mode, fn) {
  const db = await openDb();
  const tx = db.transaction(storeName, mode);
  const store = tx.objectStore(storeName);
  const result = await fn(store, tx);
  await txDone(tx);
  return result;
}

async function getByKey(storeName, key) {
  return await withStore(storeName, 'readonly', async (store) => {
    return await requestToPromise(store.get(key));
  });
}

async function putRecord(storeName, value) {
  return await withStore(storeName, 'readwrite', async (store) => {
    await requestToPromise(store.put(value));
    return null;
  });
}

async function deleteByKey(storeName, key) {
  return await withStore(storeName, 'readwrite', async (store) => {
    await requestToPromise(store.delete(key));
    return null;
  });
}

async function getAllKeys(storeName) {
  return await withStore(storeName, 'readonly', async (store) => {
    return await requestToPromise(store.getAllKeys());
  });
}

async function getChildren(parentPath) {
  return await withStore(VFS_STORE, 'readonly', async (store) => {
    const index = store.index('by_parent');
    return await requestToPromise(index.getAll(parentPath));
  });
}

async function getAllNodes() {
  return await withStore(VFS_STORE, 'readonly', async (store) => {
    return await requestToPromise(store.getAll());
  });
}

function nowMs() {
  return Date.now();
}

function normalizePath(input) {
  let path = (input || '/').trim();
  if (!path.startsWith('/')) {
    path = '/' + path;
  }
  path = path.replace(/\\+/g, '/');
  path = path.replace(/\/+/g, '/');
  if (path.length > 1 && path.endsWith('/')) {
    path = path.slice(0, -1);
  }
  const parts = [];
  for (const segment of path.split('/')) {
    if (!segment || segment === '.') continue;
    if (segment === '..') {
      parts.pop();
      continue;
    }
    parts.push(segment);
  }
  return '/' + parts.join('/');
}

function dirname(path) {
  const p = normalizePath(path);
  if (p === '/') return '/';
  const idx = p.lastIndexOf('/');
  return idx <= 0 ? '/' : p.slice(0, idx);
}

function basename(path) {
  const p = normalizePath(path);
  if (p === '/') return '/';
  const idx = p.lastIndexOf('/');
  return p.slice(idx + 1);
}

function splitSegments(path) {
  const p = normalizePath(path);
  if (p === '/') return [];
  return p.slice(1).split('/').filter(Boolean);
}

function bytesLen(text) {
  return new TextEncoder().encode(text).length;
}

function sortEntries(entries) {
  entries.sort((a, b) => {
    if (a.kind !== b.kind) {
      return a.kind === 'directory' ? -1 : 1;
    }
    return a.name.localeCompare(b.name);
  });
  return entries;
}

function vfsNodeToMetadata(node, permission = 'virtual') {
  return {
    name: node.path === '/' ? '/' : node.name,
    path: node.path,
    kind: node.kind === 'dir' ? 'directory' : 'file',
    backend: 'indexed-db-virtual',
    size: node.kind === 'file' ? (node.size ?? 0) : null,
    modified_at_unix_ms: node.modifiedAt ?? null,
    permission,
  };
}

function vfsNodeToEntry(node) {
  return {
    name: node.path === '/' ? '/' : node.name,
    path: node.path,
    kind: node.kind === 'dir' ? 'directory' : 'file',
    size: node.kind === 'file' ? (node.size ?? 0) : null,
    modified_at_unix_ms: node.modifiedAt ?? null,
  };
}

function isDescendantPath(root, candidate) {
  if (root === '/') return candidate !== '/';
  return candidate.startsWith(root + '/');
}

async function ensureVfsSeed() {
  const root = await getByKey(VFS_STORE, '/');
  if (root) return;

  const ts = nowMs();
  const seed = [
    { path: '/', parent: null, name: '', kind: 'dir', createdAt: ts, modifiedAt: ts },
    { path: '/Documents', parent: '/', name: 'Documents', kind: 'dir', createdAt: ts, modifiedAt: ts },
    { path: '/Documents/welcome.txt', parent: '/Documents', name: 'welcome.txt', kind: 'file', content: 'Virtual file system (IndexedDB)\\n\\nThis explorer works offline and mirrors the native file API shape where possible.\\n', size: 112, createdAt: ts, modifiedAt: ts },
    { path: '/Documents/todo.txt', parent: '/Documents', name: 'todo.txt', kind: 'file', content: '- Connect a local folder (File System Access API)\\n- Edit and save files\\n- Inspect metadata and permissions\\n', size: 111, createdAt: ts, modifiedAt: ts },
    { path: '/Projects', parent: '/', name: 'Projects', kind: 'dir', createdAt: ts, modifiedAt: ts },
    { path: '/Projects/notes.json', parent: '/Projects', name: 'notes.json', kind: 'file', content: JSON.stringify({ project: 'retrodesk', storage: ['indexeddb', 'cache', 'localstorage', 'memory'] }, null, 2) + '\\n', size: 0, createdAt: ts, modifiedAt: ts },
  ];

  seed[5].size = bytesLen(seed[5].content);

  await withStore(VFS_STORE, 'readwrite', async (store) => {
    for (const node of seed) {
      await requestToPromise(store.put(node));
    }
    return null;
  });
}

async function vfsGetNode(path) {
  await ensureVfsSeed();
  return await getByKey(VFS_STORE, normalizePath(path));
}

async function vfsRequireNode(path) {
  const node = await vfsGetNode(path);
  if (!node) fail(`Path not found: ${normalizePath(path)}`);
  return node;
}

async function vfsRequireDir(path) {
  const node = await vfsRequireNode(path);
  if (node.kind !== 'dir') fail(`Not a directory: ${normalizePath(path)}`);
  return node;
}

async function vfsEnsureParentDir(path) {
  const parent = dirname(path);
  const node = await vfsRequireDir(parent);
  return node;
}

async function vfsTouchParent(path) {
  const parentPath = dirname(path);
  const parent = await getByKey(VFS_STORE, parentPath);
  if (!parent || parent.kind !== 'dir') return;
  parent.modifiedAt = nowMs();
  await putRecord(VFS_STORE, parent);
}

async function vfsListDir(path) {
  const dirPath = normalizePath(path);
  const dir = await vfsRequireDir(dirPath);
  const children = await getChildren(dir.path);
  return {
    cwd: dir.path,
    backend: 'indexed-db-virtual',
    permission: 'virtual',
    entries: sortEntries((children || []).map(vfsNodeToEntry)),
  };
}

async function vfsReadText(path) {
  const node = await vfsRequireNode(path);
  if (node.kind !== 'file') fail(`Not a file: ${normalizePath(path)}`);
  const metadata = vfsNodeToMetadata(node, 'virtual');
  return {
    backend: 'indexed-db-virtual',
    path: node.path,
    text: node.content ?? '',
    metadata,
    cached_preview_key: `file-preview:${node.path}`,
  };
}

async function vfsWriteText(path, text) {
  const normalized = normalizePath(path);
  if (normalized === '/') fail('Cannot write to root');
  await vfsEnsureParentDir(normalized);
  const existing = await getByKey(VFS_STORE, normalized);
  const ts = nowMs();
  const node = existing
    ? { ...existing, kind: 'file', content: text, size: bytesLen(text), modifiedAt: ts }
    : {
        path: normalized,
        parent: dirname(normalized),
        name: basename(normalized),
        kind: 'file',
        content: text,
        size: bytesLen(text),
        createdAt: ts,
        modifiedAt: ts,
      };
  await putRecord(VFS_STORE, node);
  await vfsTouchParent(normalized);
  return vfsNodeToMetadata(node, 'virtual');
}

async function vfsCreateDir(path) {
  const normalized = normalizePath(path);
  if (normalized === '/') {
    return vfsNodeToMetadata(await vfsRequireDir('/'), 'virtual');
  }
  await vfsEnsureParentDir(normalized);
  const existing = await getByKey(VFS_STORE, normalized);
  if (existing) {
    if (existing.kind !== 'dir') fail(`File already exists at ${normalized}`);
    return vfsNodeToMetadata(existing, 'virtual');
  }
  const ts = nowMs();
  const node = {
    path: normalized,
    parent: dirname(normalized),
    name: basename(normalized),
    kind: 'dir',
    createdAt: ts,
    modifiedAt: ts,
  };
  await putRecord(VFS_STORE, node);
  await vfsTouchParent(normalized);
  return vfsNodeToMetadata(node, 'virtual');
}

async function vfsCreateFile(path, text) {
  return await vfsWriteText(path, text ?? '');
}

async function vfsDelete(path, recursive) {
  const normalized = normalizePath(path);
  if (normalized === '/') fail('Cannot delete root directory');
  const node = await vfsRequireNode(normalized);
  if (node.kind === 'dir') {
    const children = await getChildren(normalized);
    if ((children?.length ?? 0) > 0 && !recursive) {
      fail(`Directory not empty: ${normalized}`);
    }
    if (recursive) {
      const allNodes = await getAllNodes();
      const txDb = await openDb();
      const tx = txDb.transaction(VFS_STORE, 'readwrite');
      const store = tx.objectStore(VFS_STORE);
      for (const candidate of allNodes || []) {
        if (candidate.path === normalized || isDescendantPath(normalized, candidate.path)) {
          await requestToPromise(store.delete(candidate.path));
        }
      }
      await txDone(tx);
    } else {
      await deleteByKey(VFS_STORE, normalized);
    }
  } else {
    await deleteByKey(VFS_STORE, normalized);
  }
  await vfsTouchParent(normalized);
}

async function vfsStat(path) {
  const node = await vfsRequireNode(path);
  return vfsNodeToMetadata(node, 'virtual');
}

async function getNativeRootHandle() {
  const record = await getByKey(FS_CONFIG_STORE, 'native_root_handle');
  return record?.value ?? null;
}

async function setNativeRootHandle(handle) {
  await putRecord(FS_CONFIG_STORE, {
    key: 'native_root_handle',
    value: handle,
    updatedAt: nowMs(),
  });
  await putRecord(FS_CONFIG_STORE, {
    key: 'native_root_name',
    value: handle?.name ?? null,
    updatedAt: nowMs(),
  });
}

async function clearNativeRootHandle() {
  await deleteByKey(FS_CONFIG_STORE, 'native_root_handle');
  await deleteByKey(FS_CONFIG_STORE, 'native_root_name');
}

async function getNativeRootName() {
  const record = await getByKey(FS_CONFIG_STORE, 'native_root_name');
  return record?.value ?? null;
}

function mapPermission(permission) {
  if (permission === 'granted') return 'granted';
  if (permission === 'prompt') return 'prompt';
  if (permission === 'denied') return 'denied';
  return 'unsupported';
}

async function queryHandlePermission(handle, mode = 'read') {
  if (!handle) return 'prompt';
  if (typeof handle.queryPermission !== 'function') return 'unsupported';
  try {
    const result = await handle.queryPermission({ mode });
    return mapPermission(result);
  } catch {
    return 'unsupported';
  }
}

async function requestHandlePermission(handle, mode = 'read') {
  if (!handle) return 'prompt';
  if (typeof handle.requestPermission !== 'function') return 'unsupported';
  try {
    const result = await handle.requestPermission({ mode });
    return mapPermission(result);
  } catch {
    return 'denied';
  }
}

async function resolveNativeDirectoryHandle(path, opts = { create: false }) {
  const root = await getNativeRootHandle();
  if (!root) fail('No native directory is connected');
  let current = root;
  const segments = splitSegments(path);
  for (const segment of segments) {
    current = await current.getDirectoryHandle(segment, { create: !!opts.create });
  }
  return current;
}

async function resolveNativeParentAndName(path) {
  const normalized = normalizePath(path);
  if (normalized === '/') fail('Root path is not writable');
  const parentPath = dirname(normalized);
  const name = basename(normalized);
  const parent = await resolveNativeDirectoryHandle(parentPath, { create: false });
  return { normalized, parentPath, parent, name };
}

async function nativeEntryMetadata(path, handle, permission) {
  const normalized = normalizePath(path);
  if (handle.kind === 'directory') {
    return {
      name: normalized === '/' ? '/' : basename(normalized),
      path: normalized,
      kind: 'directory',
      backend: 'native-fs-access',
      size: null,
      modified_at_unix_ms: null,
      permission,
    };
  }
  const file = await handle.getFile();
  return {
    name: basename(normalized),
    path: normalized,
    kind: 'file',
    backend: 'native-fs-access',
    size: file.size,
    modified_at_unix_ms: file.lastModified ?? null,
    permission,
  };
}

async function resolveNativeFileHandle(path) {
  const { parent, name } = await resolveNativeParentAndName(path);
  return await parent.getFileHandle(name, { create: false });
}

async function resolveNativeEntry(path) {
  const normalized = normalizePath(path);
  if (normalized === '/') {
    const root = await getNativeRootHandle();
    if (!root) fail('No native directory is connected');
    return root;
  }
  const { parent, name } = await resolveNativeParentAndName(normalized);
  try {
    return await parent.getFileHandle(name, { create: false });
  } catch (_) {
    return await parent.getDirectoryHandle(name, { create: false });
  }
}

async function nativeStatus() {
  const native_supported = typeof window !== 'undefined' && typeof window.showDirectoryPicker === 'function';
  const root = native_supported ? await getNativeRootHandle() : null;
  const has_native_root = !!root;
  const permission = !native_supported
    ? 'virtual'
    : root
      ? await queryHandlePermission(root, 'readwrite')
      : 'prompt';
  const rootName = root?.name ?? (await getNativeRootName());
  return {
    backend: has_native_root ? 'native-fs-access' : 'indexed-db-virtual',
    native_supported,
    has_native_root,
    permission: has_native_root ? permission : (native_supported ? 'prompt' : 'virtual'),
    root_path_hint: rootName ? `/${rootName}` : null,
  };
}

function cacheRequestUrl(cacheName, key) {
  return `https://retrodesk.local/__cache/${encodeURIComponent(cacheName)}/${encodeURIComponent(key)}`;
}

async function cachePutTextInternal(cacheName, key, value) {
  if (typeof caches === 'undefined') {
    fail('Cache API unavailable');
  }
  const cache = await caches.open(cacheName);
  const req = new Request(cacheRequestUrl(cacheName, key), { method: 'GET' });
  const res = new Response(value, {
    headers: {
      'content-type': 'text/plain; charset=utf-8',
      'x-retrodesk-cache-key': key,
    },
  });
  await cache.put(req, res);
}

async function cacheGetTextInternal(cacheName, key) {
  if (typeof caches === 'undefined') {
    fail('Cache API unavailable');
  }
  const cache = await caches.open(cacheName);
  const req = new Request(cacheRequestUrl(cacheName, key), { method: 'GET' });
  const res = await cache.match(req);
  if (!res) return null;
  return await res.text();
}

async function cacheDeleteInternal(cacheName, key) {
  if (typeof caches === 'undefined') {
    fail('Cache API unavailable');
  }
  const cache = await caches.open(cacheName);
  const req = new Request(cacheRequestUrl(cacheName, key), { method: 'GET' });
  await cache.delete(req);
}

async function appStateLoad(namespace) {
  const row = await getByKey(APP_STATE_STORE, namespace);
  return row ?? null;
}

async function appStateSave(envelope) {
  if (!envelope || typeof envelope !== 'object') fail('Invalid app-state envelope');
  await putRecord(APP_STATE_STORE, envelope);
  return null;
}

async function appStateDelete(namespace) {
  await deleteByKey(APP_STATE_STORE, namespace);
  return null;
}

async function appStateNamespaces() {
  const keys = await getAllKeys(APP_STATE_STORE);
  return (keys || []).map(String).sort();
}

async function explorerStatus() {
  await ensureVfsSeed();
  return await nativeStatus();
}

async function explorerPickNativeDirectory() {
  if (typeof window === 'undefined' || typeof window.showDirectoryPicker !== 'function') {
    fail('File System Access API is not supported in this browser');
  }
  const handle = await window.showDirectoryPicker({ mode: 'readwrite' });
  await setNativeRootHandle(handle);
  await requestHandlePermission(handle, 'readwrite');
  return await nativeStatus();
}

async function explorerRequestPermission(mode) {
  const status = await nativeStatus();
  if (status.backend !== 'native-fs-access') {
    return 'virtual';
  }
  const root = await getNativeRootHandle();
  return await requestHandlePermission(root, mode === 'readwrite' ? 'readwrite' : 'read');
}

async function explorerListDir(path) {
  await ensureVfsSeed();
  const status = await nativeStatus();
  if (status.backend !== 'native-fs-access') {
    return await vfsListDir(path);
  }
  const root = await getNativeRootHandle();
  const permission = await queryHandlePermission(root, 'read');
  if (permission === 'denied') {
    fail('Native folder permission denied');
  }
  const dir = await resolveNativeDirectoryHandle(path, { create: false });
  const entries = [];
  for await (const [name, handle] of dir.entries()) {
    const entryPath = normalizePath(`${normalizePath(path)}/${name}`);
    if (handle.kind === 'directory') {
      entries.push({
        name,
        path: entryPath,
        kind: 'directory',
        size: null,
        modified_at_unix_ms: null,
      });
    } else {
      const file = await handle.getFile();
      entries.push({
        name,
        path: entryPath,
        kind: 'file',
        size: file.size,
        modified_at_unix_ms: file.lastModified ?? null,
      });
    }
  }
  sortEntries(entries);
  return {
    cwd: normalizePath(path),
    backend: 'native-fs-access',
    permission,
    entries,
  };
}

async function explorerReadTextFile(path) {
  await ensureVfsSeed();
  const status = await nativeStatus();
  if (status.backend !== 'native-fs-access') {
    const result = await vfsReadText(path);
    await cachePutTextInternal('retrodesk-explorer-cache-v1', result.cached_preview_key, result.text);
    return result;
  }
  const root = await getNativeRootHandle();
  const permission = await queryHandlePermission(root, 'read');
  if (permission === 'denied') fail('Native folder permission denied');
  const normalized = normalizePath(path);
  const fileHandle = await resolveNativeFileHandle(normalized);
  const file = await fileHandle.getFile();
  const text = await file.text();
  const metadata = await nativeEntryMetadata(normalized, fileHandle, permission);
  const cached_preview_key = `file-preview:${normalized}`;
  await cachePutTextInternal('retrodesk-explorer-cache-v1', cached_preview_key, text);
  return {
    backend: 'native-fs-access',
    path: normalized,
    text,
    metadata,
    cached_preview_key,
  };
}

async function explorerWriteTextFile(path, text) {
  await ensureVfsSeed();
  const status = await nativeStatus();
  if (status.backend !== 'native-fs-access') {
    const meta = await vfsWriteText(path, text ?? '');
    await cachePutTextInternal('retrodesk-explorer-cache-v1', `file-preview:${meta.path}`, text ?? '');
    return meta;
  }
  const root = await getNativeRootHandle();
  const permission = await requestHandlePermission(root, 'readwrite');
  if (permission !== 'granted') fail('Write permission is required to save files');
  const normalized = normalizePath(path);
  const { parent, name } = await resolveNativeParentAndName(normalized);
  const fileHandle = await parent.getFileHandle(name, { create: true });
  const writable = await fileHandle.createWritable();
  await writable.write(text ?? '');
  await writable.close();
  const metadata = await nativeEntryMetadata(normalized, fileHandle, permission);
  await cachePutTextInternal('retrodesk-explorer-cache-v1', `file-preview:${normalized}`, text ?? '');
  return metadata;
}

async function explorerCreateDir(path) {
  await ensureVfsSeed();
  const status = await nativeStatus();
  if (status.backend !== 'native-fs-access') {
    return await vfsCreateDir(path);
  }
  const root = await getNativeRootHandle();
  const permission = await requestHandlePermission(root, 'readwrite');
  if (permission !== 'granted') fail('Write permission is required to create folders');
  const normalized = normalizePath(path);
  if (normalized === '/') {
    const rootHandle = await getNativeRootHandle();
    return await nativeEntryMetadata('/', rootHandle, permission);
  }
  const segments = splitSegments(normalized);
  let current = await getNativeRootHandle();
  for (const segment of segments) {
    current = await current.getDirectoryHandle(segment, { create: true });
  }
  return await nativeEntryMetadata(normalized, current, permission);
}

async function explorerCreateFile(path, text) {
  return await explorerWriteTextFile(path, text ?? '');
}

async function explorerDelete(path, recursive) {
  await ensureVfsSeed();
  const status = await nativeStatus();
  if (status.backend !== 'native-fs-access') {
    await vfsDelete(path, !!recursive);
    return null;
  }
  const root = await getNativeRootHandle();
  const permission = await requestHandlePermission(root, 'readwrite');
  if (permission !== 'granted') fail('Write permission is required to delete entries');
  const { parent, name, normalized } = await resolveNativeParentAndName(path);
  if (normalized === '/') fail('Cannot delete root directory');
  await parent.removeEntry(name, { recursive: !!recursive });
  await cacheDeleteInternal('retrodesk-explorer-cache-v1', `file-preview:${normalized}`).catch(() => {});
  return null;
}

async function explorerStat(path) {
  await ensureVfsSeed();
  const status = await nativeStatus();
  if (status.backend !== 'native-fs-access') {
    return await vfsStat(path);
  }
  const root = await getNativeRootHandle();
  const permission = await queryHandlePermission(root, 'read');
  if (permission === 'denied') fail('Native folder permission denied');
  const handle = await resolveNativeEntry(path);
  return await nativeEntryMetadata(path, handle, permission);
}

export async function jsAppStateLoad(namespace) { return await appStateLoad(namespace); }
export async function jsAppStateSave(envelope) { return await appStateSave(envelope); }
export async function jsAppStateDelete(namespace) { return await appStateDelete(namespace); }
export async function jsAppStateNamespaces() { return await appStateNamespaces(); }

export async function jsCachePutText(cacheName, key, value) { return await cachePutTextInternal(cacheName, key, value); }
export async function jsCacheGetText(cacheName, key) { return await cacheGetTextInternal(cacheName, key); }
export async function jsCacheDelete(cacheName, key) { return await cacheDeleteInternal(cacheName, key); }

export async function jsExplorerStatus() { return await explorerStatus(); }
export async function jsExplorerPickNativeDirectory() { return await explorerPickNativeDirectory(); }
export async function jsExplorerRequestPermission(mode) { return await explorerRequestPermission(mode); }
export async function jsExplorerListDir(path) { return await explorerListDir(path); }
export async function jsExplorerReadTextFile(path) { return await explorerReadTextFile(path); }
export async function jsExplorerWriteTextFile(path, text) { return await explorerWriteTextFile(path, text); }
export async function jsExplorerCreateDir(path) { return await explorerCreateDir(path); }
export async function jsExplorerCreateFile(path, text) { return await explorerCreateFile(path, text); }
export async function jsExplorerDelete(path, recursive) { return await explorerDelete(path, recursive); }
export async function jsExplorerStat(path) { return await explorerStat(path); }
export async function jsExplorerClearNativeRoot() { await clearNativeRootHandle(); return await nativeStatus(); }
