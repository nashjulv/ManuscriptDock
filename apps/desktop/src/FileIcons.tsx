export function FileIcon({ name, folder = false }: { name: string; folder?: boolean }) {
  if (folder) return <svg className="native-file-icon folder-icon" viewBox="0 0 48 40" aria-hidden="true"><path d="M3 8a4 4 0 0 1 4-4h11l5 5h18a4 4 0 0 1 4 4v21a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4Z" fill="#7abbdc"/><path d="M3 15a3 3 0 0 1 3-3h36a3 3 0 0 1 3 3v19a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4Z" fill="#a2d5ed"/><path d="M6 13h36" stroke="#d5effb" strokeWidth="1.5"/></svg>;
  const extension = name.split('.').pop()?.toUpperCase() ?? '';
  const color = /DOC|DOCX/.test(extension) ? '#497abc' : /XLS|XLSX|CSV/.test(extension) ? '#45866b' : extension === 'PDF' ? '#c36562' : /PNG|JPG|JPEG|TIFF/.test(extension) ? '#9474b4' : '#81909d';
  return <svg className="native-file-icon document-icon" viewBox="0 0 40 48" aria-hidden="true"><path d="M6 2h19l10 10v32a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2Z" fill="#fff" stroke="#ccd2d6"/><path d="M25 2v10h10" fill="#edf0f3" stroke="#ccd2d6"/><path d="M11 20h17M11 25h17M11 30h12" stroke="#d7dee3" strokeWidth="1.5"/><rect x="0" y="33" width="35" height="12" rx="3" fill={color}/><text x="17.5" y="41.7" textAnchor="middle" fontFamily="system-ui,sans-serif" fontSize="7.5" fontWeight="650" fill="white">{extension.slice(0, 5) || 'FILE'}</text></svg>;
}

export function BrowserIcon({ name }: { name: 'back' | 'up' | 'grid' | 'list' | 'refresh' | 'external' | 'add' | 'chevron' }) {
  const paths = { back: 'm14 5-7 7 7 7', up: 'm5 13 7-7 7 7M12 6v14', grid: 'M4 4h6v6H4ZM14 4h6v6h-6ZM4 14h6v6H4ZM14 14h6v6h-6Z', list: 'M4 6h2M10 6h10M4 12h2M10 12h10M4 18h2M10 18h10', refresh: 'M20 11a8 8 0 1 0-2 6M20 4v7h-7', external: 'M14 4h6v6M20 4l-9 9M10 4H5a1 1 0 0 0-1 1v14a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-5', add: 'M12 5v14M5 12h14', chevron: 'm9 5 7 7-7 7' };
  return <svg className="browser-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={paths[name]}/></svg>;
}
