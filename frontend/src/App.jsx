import React, { useState, useRef } from 'react';
import DocumentPreview from './components/DocumentPreview';

function App() {
  const [file, setFile] = useState(null);
  const [srcLang, setSrcLang] = useState('en');
  const [tgtLang, setTgtLang] = useState('ne');
  const [isTranslating, setIsTranslating] = useState(false);
  const [translatedUrl, setTranslatedUrl] = useState(null);
  const [translatedFileName, setTranslatedFileName] = useState('');
  const [translatedFile, setTranslatedFile] = useState(null);
  const [isDragOver, setIsDragOver] = useState(false);
  const [isSidebarOpen, setIsSidebarOpen] = useState(true);
  const [leftWidthPct, setLeftWidthPct] = useState(50);
  const [isDraggingResizer, setIsDraggingResizer] = useState(false);
  const [theme, setTheme] = useState(document.documentElement.classList.contains('dark') ? 'dark' : 'light');

  React.useEffect(() => {
    if (theme === 'dark') {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }, [theme]);

  const toggleTheme = () => {
    setTheme(prev => prev === 'dark' ? 'light' : 'dark');
  };

  const fileInputRef = useRef(null);
  const leftPanelRef = useRef(null);

  const handleMouseDownResizer = (e) => {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = leftWidthPct;
    const container = document.getElementById('preview-container');
    
    if (!container || !leftPanelRef.current) return;
    const containerWidth = container.getBoundingClientRect().width;
    
    setIsDraggingResizer(true);

    const onMouseMove = (moveEvent) => {
      const deltaX = moveEvent.clientX - startX;
      let newWidth = startWidth + (deltaX / containerWidth) * 100;
      newWidth = Math.min(Math.max(newWidth, 20), 80);
      
      if (leftPanelRef.current) {
        leftPanelRef.current.style.flex = `0 0 calc(${newWidth}% - 8px)`;
      }
    };

    const onMouseUp = (upEvent) => {
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
      document.body.style.cursor = '';
      
      const deltaX = upEvent.clientX - startX;
      let newWidth = startWidth + (deltaX / containerWidth) * 100;
      newWidth = Math.min(Math.max(newWidth, 20), 80);
      setLeftWidthPct(newWidth);
      setIsDraggingResizer(false);
    };

    document.body.style.cursor = 'col-resize';
    document.addEventListener('mousemove', onMouseMove);
    document.addEventListener('mouseup', onMouseUp);
  };

  const handleDragOver = (e) => {
    e.preventDefault();
    setIsDragOver(true);
  };

  const handleDragLeave = () => {
    setIsDragOver(false);
  };

  const handleDrop = (e) => {
    e.preventDefault();
    setIsDragOver(false);
    if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
      handleFileSelect(e.dataTransfer.files[0]);
    }
  };

  const handleFileInputChange = (e) => {
    if (e.target.files && e.target.files.length > 0) {
      handleFileSelect(e.target.files[0]);
    }
  };

  const handleFileSelect = (selectedFile) => {
    setFile(selectedFile);
    setTranslatedUrl(null);
    setTranslatedFileName('');
    setTranslatedFile(null);
  };

  const loadSampleFile = async (filePath, filename) => {
    try {
      let mimeType = 'text/plain';
      if(filename.endsWith('.pdf')) mimeType = 'application/pdf';
      if(filename.endsWith('.csv')) mimeType = 'text/csv';
      if(filename.endsWith('.docx')) mimeType = 'application/vnd.openxmlformats-officedocument.wordprocessingml.document';

      const response = await fetch(`/${filePath}`);
      if (!response.ok) throw new Error('Could not fetch test file');
      
      const blob = await response.blob();
      const loadedFile = new File([blob], filename, { type: mimeType });
      handleFileSelect(loadedFile);
    } catch (err) {
      console.error(err);
      alert("Failed to load sample file. Make sure they exist in the public directory.");
    }
  };

  const handleTranslate = async (e) => {
    e.preventDefault();
    if (!file) {
      alert('Please select a file');
      return;
    }

    setIsTranslating(true);
    setTranslatedUrl(null);
    setTranslatedFile(null);
    setTranslatedFile(null);

    const formData = new FormData();
    formData.append('file', file);
    formData.append('src', srcLang);
    formData.append('tgt', tgtLang);

    try {
      const response = await fetch('/translate', {
        method: 'POST',
        body: formData
      });

      if (!response.ok) throw new Error('Translation failed');

      const blob = await response.blob();
      const url = window.URL.createObjectURL(blob);
      
      let filename = `translated_${file.name}`;
      const contentDisposition = response.headers.get('Content-Disposition');
      if (contentDisposition && contentDisposition.includes('filename=')) {
        filename = contentDisposition.split('filename=')[1].replace(/["']/g, '');
      } else {
        const extIndex = file.name.lastIndexOf('.');
        if (extIndex !== -1) {
          filename = `${file.name.substring(0, extIndex)}_${tgtLang}${file.name.substring(extIndex)}`;
        }
      }

      setTranslatedUrl(url);
      setTranslatedFileName(filename);
      setTranslatedFile(new File([blob], filename, { type: blob.type }));


    } catch (error) {
      console.error(error);
      alert(error.message);
    } finally {
      setIsTranslating(false);
    }
  };

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-gray-50 dark:bg-[#0a0a0a] text-gray-900 dark:text-white relative transition-colors duration-300">
      {/* Sidebar Backdrop (Mobile only) */}
      {isSidebarOpen && (
        <div 
          className="md:hidden fixed inset-0 bg-black/60 backdrop-blur-sm z-[90]"
          onClick={() => setIsSidebarOpen(false)}
        />
      )}

      {/* Sidebar Toggle (Only visible when sidebar is closed) */}
      {!isSidebarOpen && (
        <button 
          onClick={() => setIsSidebarOpen(true)}
          className="absolute top-3 left-4 z-[110] bg-white dark:bg-white/5 hover:bg-gray-100 dark:hover:bg-white/10 text-gray-800 dark:text-white p-2.5 rounded-[6px] cursor-pointer backdrop-blur-md border border-gray-200 dark:border-white/10 transition-all active:scale-95 shadow-lg dark:shadow-2xl flex items-center justify-center"
        >
          <span className="material-symbols-outlined block text-[24px]">menu</span>
        </button>
      )}

      {/* Left Sidebar */}
      <aside 
        className={`
          ${isSidebarOpen ? 'translate-x-0 w-[300px] md:w-[320px]' : '-translate-x-full w-0'} 
          fixed md:relative inset-y-0 left-0 transition-all duration-300 ease-in-out shrink-0 h-screen flex flex-col border-r border-gray-200 dark:border-white/10 bg-white/90 md:bg-white/60 dark:bg-black/80 dark:md:bg-black/40 backdrop-blur-[40px] z-[100] overflow-hidden shadow-xl md:shadow-none
        `}
      >
        <div className="p-4 flex flex-col gap-4 min-w-[300px] md:min-w-[320px] h-full overflow-y-auto">
          <div className="flex items-center justify-between">
            <img src="/logoanubadh.png" alt="Aanubadh" className="h-12 rounded-lg object-contain" />
            <div className="flex items-center gap-2">
              <button 
                onClick={toggleTheme}
                className="text-gray-500 hover:text-gray-800 dark:text-slate-400 dark:hover:text-white p-2 hover:bg-gray-100 dark:hover:bg-white/5 rounded-[6px] border border-transparent dark:hover:border-white/10 transition-colors flex items-center cursor-pointer justify-center"
                title={theme === 'dark' ? "Switch to Light Mode" : "Switch to Dark Mode"}
              >
                <span className="material-symbols-outlined text-[24px]">
                  {theme === 'dark' ? 'light_mode' : 'dark_mode'}
                </span>
              </button>
              <button 
                onClick={() => setIsSidebarOpen(false)}
                className="text-gray-500 hover:text-gray-800 dark:text-slate-400 dark:hover:text-white p-2 hover:bg-gray-100 dark:hover:bg-white/5 rounded-[6px] border border-transparent dark:hover:border-white/10 transition-colors flex items-center cursor-pointer justify-center"
              >
                <span className="material-symbols-outlined text-[24px]">menu_open</span>
              </button>
            </div>
          </div>

          {/* Upload Zone */}
          <form onSubmit={handleTranslate} className="flex flex-col gap-3">
            <div 
              className={`upload-zone rounded-xl p-12 flex flex-col items-center justify-center text-center cursor-pointer relative ${isDragOver ? 'dragover' : ''}`}
              onDragOver={handleDragOver}
              onDragLeave={handleDragLeave}
              onDrop={handleDrop}
              onClick={() => fileInputRef.current.click()}
            >
              <span className="material-symbols-outlined text-3xl text-gray-400 dark:text-slate-400 mb-1">cloud_upload</span>
              <span className="font-medium text-gray-700 dark:text-slate-200 text-sm text-balance">Drag & Drop File</span>
              <span className="text-xs text-gray-500 dark:text-slate-500 mt-1 truncate max-w-full">{file ? file.name : "or click to browse"}</span>
              <input 
                type="file" 
                ref={fileInputRef}
                className="hidden" 
                accept=".csv,.docx,.pdf,.txt" 
                onChange={handleFileInputChange}
              />
            </div>

            <div className="flex flex-col gap-2">
              <div>
                <label className="text-xs text-gray-500 dark:text-slate-400 uppercase tracking-widest">From</label>
                <select 
                  value={srcLang} 
                  onChange={(e) => {
                    setSrcLang(e.target.value);
                    if (tgtLang === e.target.value) {
                      setTgtLang(e.target.value === 'en' ? 'ne' : 'en');
                    }
                  }}
                  className="w-full mt-1 bg-gray-50 dark:bg-[#1a1a1a] border border-gray-200 dark:border-white/10 rounded-lg px-3 py-2 text-gray-900 dark:text-white appearance-none focus:outline-none focus:border-blue-500 dark:focus:border-blue-500 text-sm transition-colors"
                >
                  <option value="en">English</option>
                  <option value="ne">Nepali</option>
                  <option value="tmg">Tamang</option>
                </select>
              </div>
              <div>
                <label className="text-xs text-gray-500 dark:text-slate-400 uppercase tracking-widest">To</label>
                <select 
                  value={tgtLang} 
                  onChange={(e) => setTgtLang(e.target.value)}
                  className="w-full mt-1 bg-gray-50 dark:bg-[#1a1a1a] border border-gray-200 dark:border-white/10 rounded-lg px-3 py-2 text-gray-900 dark:text-white appearance-none focus:outline-none focus:border-blue-500 dark:focus:border-blue-500 text-sm transition-colors"
                >
                  <option value="ne" disabled={srcLang === 'ne'} hidden={srcLang === 'ne'}>Nepali</option>
                  <option value="en" disabled={srcLang === 'en'} hidden={srcLang === 'en'}>English</option>
                  <option value="tmg" disabled={srcLang === 'tmg'} hidden={srcLang === 'tmg'}>Tamang</option>
                </select>
              </div>
            </div>

            <button 
              type="submit" 
              className="w-full bg-blue-600 hover:bg-blue-700 text-white rounded-lg py-2.5 font-bold transition-colors shadow-lg mt-1 flex items-center justify-center gap-2 disabled:opacity-50"
              disabled={isTranslating}
            >
              {isTranslating ? 'Translating...' : 'Translate'}
            </button>
          </form>

          {/* Test Files section */}
          <div className="pt-4 border-t border-gray-200 dark:border-white/10 mt-1">
            <h3 className="text-xs text-gray-500 dark:text-zinc-500 uppercase tracking-widest mb-2">Test Files</h3>
            <div className="flex flex-col gap-1.5">
              <button onClick={() => loadSampleFile('test_files/sample_pdf.pdf', 'sample_pdf.pdf')} className="flex items-center gap-3 text-sm text-gray-600 dark:text-zinc-400 hover:text-gray-900 hover:bg-gray-100 dark:hover:text-white dark:hover:bg-white/5 rounded-lg p-2 transition-colors text-left w-full">
                <span className="material-symbols-outlined text-[18px] text-red-500">picture_as_pdf</span>
                <div className="flex flex-col overflow-hidden">
                  <span className="font-medium text-gray-800 dark:text-slate-300 truncate">sample_pdf.pdf</span>
                  <span className="text-[10px] text-gray-500 dark:text-zinc-500">Test PDF</span>
                </div>
              </button>
              <button onClick={() => loadSampleFile('test_files/sample_csv.csv', 'sample_csv.csv')} className="flex items-center gap-3 text-sm text-gray-600 dark:text-zinc-400 hover:text-gray-900 hover:bg-gray-100 dark:hover:text-white dark:hover:bg-white/5 rounded-lg p-2 transition-colors text-left w-full">
                <span className="material-symbols-outlined text-[18px] text-green-500">table_chart</span>
                <div className="flex flex-col overflow-hidden">
                  <span className="font-medium text-gray-800 dark:text-slate-300 truncate">sample_csv.csv</span>
                  <span className="text-[10px] text-gray-500 dark:text-zinc-500">Test CSV</span>
                </div>
              </button>
              <button onClick={() => loadSampleFile('test_files/sample_docx.docx', 'sample_docx.docx')} className="flex items-center gap-3 text-sm text-gray-600 dark:text-zinc-400 hover:text-gray-900 hover:bg-gray-100 dark:hover:text-white dark:hover:bg-white/5 rounded-lg p-2 transition-colors text-left w-full">
                <span className="material-symbols-outlined text-[18px] text-blue-500">description</span>
                <div className="flex flex-col overflow-hidden">
                  <span className="font-medium text-gray-800 dark:text-slate-300 truncate">sample_docx.docx</span>
                  <span className="text-[10px] text-gray-500 dark:text-zinc-500">Test DOCX</span>
                </div>
              </button>
              <button onClick={() => loadSampleFile('test_files/sample.txt', 'sample.txt')} className="flex items-center gap-3 text-sm text-gray-600 dark:text-zinc-400 hover:text-gray-900 hover:bg-gray-100 dark:hover:text-white dark:hover:bg-white/5 rounded-lg p-2 transition-colors text-left w-full">
                <span className="material-symbols-outlined text-[18px] text-gray-500">article</span>
                <div className="flex flex-col overflow-hidden">
                  <span className="font-medium text-gray-800 dark:text-slate-300 truncate">sample.txt</span>
                  <span className="text-[10px] text-gray-500 dark:text-zinc-500">Test TXT</span>
                </div>
              </button>
            </div>
          </div>
        </div>
      </aside>

      {/* Main Live Preview Area */}
      <main className={`flex-1 min-w-0 flex flex-col h-screen relative p-4 md:p-6 bg-gray-50 dark:bg-[#0a0a0a] transition-colors duration-300 ${!file ? 'items-center justify-center' : 'overflow-y-auto md:overflow-hidden'}`}>
        
        {!file && (
          <div className="flex flex-col items-center text-center text-gray-400 dark:text-zinc-500">
            <span className="material-symbols-outlined text-6xl mb-4 opacity-50">visibility</span>
            <h2 className="text-xl font-medium text-gray-500 dark:text-zinc-400">Live Preview</h2>
            <p className="text-sm mt-2 px-6">Upload or select a test file to preview it here.</p>
          </div>
        )}

        {file && (
          <div className="w-full h-full flex flex-col gap-4 min-h-0 min-w-0">
            <div className="flex justify-center items-center text-gray-700 dark:text-zinc-300 px-2 md:px-0">
              <span className="font-semibold text-lg truncate pr-4">{file.name}</span>
            </div>
            
            <div id="preview-container" className="flex-1 flex w-full relative flex-col md:flex-row gap-4 md:gap-0 min-h-0 pb-12 md:pb-0">
              {isDraggingResizer && (
                <div className="absolute inset-0 z-50 cursor-col-resize"></div>
              )}
              {/* Original Preview */}
              <div 
                ref={leftPanelRef}
                className="doc-preview-wrapper relative flex flex-col bg-white dark:bg-[#1a1a1a] border border-gray-200 dark:border-white/10 rounded-xl overflow-hidden h-[500px] md:h-auto md:min-h-0 p-1 md:p-0 shadow-sm dark:shadow-none"
                style={
                  translatedUrl && translatedFile 
                    ? (window.innerWidth >= 768 ? { flex: `0 0 calc(${leftWidthPct}% - 8px)` } : { flex: '0 0 500px' })
                    : { flex: 1, minWidth: 0, minHeight: 0 }
                }
              >
                <div className="h-12 md:h-12 px-4 border-b border-gray-200 dark:border-white/10 flex justify-between items-center bg-gray-50 dark:bg-[#151515] z-20 shrink-0">
                  <span className="text-sm font-semibold text-gray-700 dark:text-zinc-300">Original</span>
                </div>
                <div className="flex-1 overflow-hidden relative">
                  <DocumentPreview file={file} />
                </div>
              </div>

              {/* Resizer - Hidden on Mobile */}
              {translatedUrl && translatedFile && (
                <div 
                  className="hidden md:flex w-4 cursor-col-resize items-center justify-center hover:bg-gray-200 dark:hover:bg-white/5 transition-colors z-30 group"
                  onMouseDown={handleMouseDownResizer}
                >
                  <div className="w-1 h-12 bg-gray-300 dark:bg-white/10 group-hover:bg-blue-600 dark:group-hover:bg-blue-500 rounded-full transition-colors"></div>
                </div>
              )}

              {/* Result Area */}
              {translatedUrl && translatedFile && (
                <div className="flex-1 min-w-0 h-[500px] md:h-auto md:min-h-0 doc-preview-wrapper relative flex flex-col bg-white dark:bg-[#1a1a1a] border border-gray-200 dark:border-white/10 rounded-xl overflow-hidden shrink-0 p-1 md:p-0 shadow-sm dark:shadow-none">
                  <div className="h-12 md:h-12 px-4 border-b border-gray-200 dark:border-white/10 flex justify-between items-center bg-gray-50 dark:bg-[#151515] z-20 shrink-0">
                    <span className="text-sm font-semibold text-blue-600 dark:text-blue-400">Translated</span>
                    <a href={translatedUrl} download={translatedFileName} className="inline-flex items-center gap-1 bg-blue-600 hover:bg-blue-700 text-white rounded px-3 py-1.5 text-xs font-bold transition-colors shadow-sm">
                      <span className="material-symbols-outlined text-[16px]">download</span> Download
                    </a>
                  </div>
                  <div className="flex-1 overflow-hidden relative pb-2">
                    <DocumentPreview file={translatedFile} />
                  </div>
                </div>
              )}
            </div>
          </div>
        )}

        {/* Progress Overlay */}
        {isTranslating && (
          <div className="absolute inset-0 bg-white/80 dark:bg-black/80 backdrop-blur-sm z-50 flex flex-col items-center justify-center">
            <div className="relative w-24 h-24 flex items-center justify-center mb-6">
              <div className="absolute inset-0 rounded-full border border-blue-600 dark:border-blue-500 animate-ping opacity-50"></div>
              <span className="text-4xl font-bold bg-gradient-to-b from-gray-900 to-blue-600 dark:from-white dark:to-blue-500 bg-clip-text text-transparent">अ</span>
            </div>
            <h2 className="text-xl md:text-2xl font-bold text-gray-900 dark:text-white mb-2">Translating...</h2>
            <div className="w-48 md:w-64 bg-gray-200 dark:bg-white/10 h-1.5 rounded-full mt-4 overflow-hidden">
              <div className="bg-blue-600 dark:bg-blue-500 h-full transition-all duration-300 w-1/2 animate-pulse"></div>
            </div>
          </div>
        )}

      </main>
    </div>
  );
}

export default App;
