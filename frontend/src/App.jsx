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
  const [translatedPublicUrl, setTranslatedPublicUrl] = useState(null);
  const [isDragOver, setIsDragOver] = useState(false);
  const [leftWidthPct, setLeftWidthPct] = useState(50);
  const [isDraggingResizer, setIsDraggingResizer] = useState(false);

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
    setTranslatedPublicUrl(null);
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
    setTranslatedPublicUrl(null);

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

      // Extract public file path for Microsoft Viewer (DOCX only)
      const translatedFilePath = response.headers.get('X-Translated-File-Path');
      if (translatedFilePath && filename.endsWith('.docx')) {
        const publicUrl = `${window.location.origin}${translatedFilePath}`;
        setTranslatedPublicUrl(publicUrl);
      }

    } catch (error) {
      console.error(error);
      alert(error.message);
    } finally {
      setIsTranslating(false);
    }
  };

  return (
    <div className="flex h-screen w-screen overflow-hidden">
      {/* Left Sidebar */}
      <aside className="w-[320px] shrink-0 h-screen flex flex-col border-r border-white/10 bg-black/40 backdrop-blur-[40px] z-50 overflow-y-auto">
        <div className="p-6 flex flex-col gap-6">
          <div className="flex items-center">
            <img src="/logo.jpeg" alt="Aanubadh" className="h-16 rounded-lg object-contain" />
          </div>

          {/* Upload Zone */}
          <form onSubmit={handleTranslate} className="flex flex-col gap-4">
            <div 
              className={`upload-zone rounded-xl p-6 flex flex-col items-center justify-center text-center cursor-pointer relative ${isDragOver ? 'dragover' : ''}`}
              onDragOver={handleDragOver}
              onDragLeave={handleDragLeave}
              onDrop={handleDrop}
              onClick={() => fileInputRef.current.click()}
            >
              <span className="material-symbols-outlined text-3xl text-slate-400 mb-2">cloud_upload</span>
              <span className="font-medium text-slate-200 text-sm">Drag & Drop File</span>
              <span className="text-xs text-slate-500 mt-1">{file ? file.name : "or click to browse"}</span>
              <input 
                type="file" 
                ref={fileInputRef}
                className="hidden" 
                accept=".csv,.docx,.pdf,.txt" 
                onChange={handleFileInputChange}
              />
            </div>

            {/* Languages */}
            <div className="flex flex-col gap-3">
              <div>
                <label className="text-xs text-slate-400 uppercase tracking-widest">From</label>
                <select 
                  value={srcLang} 
                  onChange={(e) => {
                    setSrcLang(e.target.value);
                    if (tgtLang === e.target.value) {
                      setTgtLang(e.target.value === 'en' ? 'ne' : 'en');
                    }
                  }}
                  className="w-full mt-1 bg-[#1a1a1a] border border-white/10 rounded-lg px-3 py-2 text-white appearance-none focus:outline-none focus:border-violet-500 text-sm"
                >
                  <option value="en">English</option>
                  <option value="ne">Nepali</option>
                  <option value="tmg">Tamang</option>
                </select>
              </div>
              <div>
                <label className="text-xs text-slate-400 uppercase tracking-widest">To</label>
                <select 
                  value={tgtLang} 
                  onChange={(e) => setTgtLang(e.target.value)}
                  className="w-full mt-1 bg-[#1a1a1a] border border-white/10 rounded-lg px-3 py-2 text-white appearance-none focus:outline-none focus:border-violet-500 text-sm"
                >
                  <option value="ne" disabled={srcLang === 'ne'} hidden={srcLang === 'ne'}>Nepali</option>
                  <option value="en" disabled={srcLang === 'en'} hidden={srcLang === 'en'}>English</option>
                  <option value="tmg" disabled={srcLang === 'tmg'} hidden={srcLang === 'tmg'}>Tamang</option>
                </select>
              </div>
            </div>

            <button 
              type="submit" 
              className="w-full bg-[#8B5CF6] hover:bg-[#7C3AED] text-white rounded-lg py-3 font-bold transition-colors shadow-lg mt-2 flex items-center justify-center gap-2 disabled:opacity-50"
              disabled={isTranslating}
            >
              {isTranslating ? 'Translating...' : 'Translate'}
            </button>
          </form>

          {/* Test Files section */}
          <div className="pt-6 border-t border-white/10 mt-2">
            <h3 className="text-xs text-zinc-500 uppercase tracking-widest mb-3">Test Files</h3>
            <div className="flex flex-col gap-2">
              <button onClick={() => loadSampleFile('test_files/sample_pdf.pdf', 'sample_pdf.pdf')} className="flex items-center gap-3 text-sm text-zinc-400 hover:text-white hover:bg-white/5 rounded-lg p-3 transition-colors text-left w-full">
                <span className="material-symbols-outlined text-[18px]">picture_as_pdf</span>
                <div className="flex flex-col">
                  <span className="font-medium text-slate-300">sample_pdf.pdf</span>
                  <span className="text-[10px] text-zinc-500">Test PDF</span>
                </div>
              </button>
              <button onClick={() => loadSampleFile('test_files/sample_csv.csv', 'sample_csv.csv')} className="flex items-center gap-3 text-sm text-zinc-400 hover:text-white hover:bg-white/5 rounded-lg p-3 transition-colors text-left w-full">
                <span className="material-symbols-outlined text-[18px]">table_chart</span>
                <div className="flex flex-col">
                  <span className="font-medium text-slate-300">sample_csv.csv</span>
                  <span className="text-[10px] text-zinc-500">Test CSV</span>
                </div>
              </button>
              <button onClick={() => loadSampleFile('test_files/sample_docx.docx', 'sample_docx.docx')} className="flex items-center gap-3 text-sm text-zinc-400 hover:text-white hover:bg-white/5 rounded-lg p-3 transition-colors text-left w-full">
                <span className="material-symbols-outlined text-[18px]">description</span>
                <div className="flex flex-col">
                  <span className="font-medium text-slate-300">sample_docx.docx</span>
                  <span className="text-[10px] text-zinc-500">Test DOCX</span>
                </div>
              </button>
            </div>
          </div>
        </div>
      </aside>

      {/* Main Live Preview Area */}
      <main className="flex-1 min-w-0 flex flex-col h-screen relative items-center justify-center p-6 bg-[#0a0a0a]">
        
        {!file && (
          <div className="flex flex-col items-center text-center text-zinc-500">
            <span className="material-symbols-outlined text-6xl mb-4 opacity-50">visibility</span>
            <h2 className="text-xl font-medium text-zinc-400">Live Preview</h2>
            <p className="text-sm mt-2">Upload or select a test file to preview it here.</p>
          </div>
        )}

        {file && (
          <div className="w-full h-full flex flex-col gap-4 min-h-0 min-w-0">
            <div className="flex justify-between items-center text-zinc-300">
              <span className="font-semibold text-lg">{file.name}</span>
            </div>
            
            <div id="preview-container" className="flex-1 flex h-full overflow-hidden w-full relative flex-row">
              {isDraggingResizer && (
                <div className="absolute inset-0 z-50 cursor-col-resize"></div>
              )}
              {/* Original Preview */}
              <div 
                ref={leftPanelRef}
                className="doc-preview-wrapper relative flex flex-col bg-[#1a1a1a] border border-white/10 rounded-xl overflow-hidden"
                style={
                  translatedUrl && translatedFile 
                    ? { flex: `0 0 calc(${leftWidthPct}% - 8px)` } 
                    : { flex: 1, minWidth: 0, minHeight: 0 }
                }
              >
                <div className="h-14 px-4 border-b border-white/10 flex justify-between items-center bg-[#151515] z-20 shrink-0">
                  <span className="text-sm font-semibold text-zinc-300">Original</span>
                </div>
                <div className="flex-1 overflow-hidden relative">
                  <DocumentPreview file={file} />
                </div>
              </div>

              {/* Resizer */}
              {translatedUrl && translatedFile && (
                <div 
                  className="w-4 cursor-col-resize flex items-center justify-center hover:bg-white/5 transition-colors z-30 group"
                  onMouseDown={handleMouseDownResizer}
                >
                  <div className="w-1 h-12 bg-white/10 group-hover:bg-[#8B5CF6] rounded-full transition-colors"></div>
                </div>
              )}

              {/* Result Area */}
              {translatedUrl && translatedFile && (
                <div className="flex-1 min-w-0 min-h-0 doc-preview-wrapper relative flex flex-col bg-[#1a1a1a] border border-white/10 rounded-xl overflow-hidden">
                  <div className="h-14 px-4 border-b border-white/10 flex justify-between items-center bg-[#151515] z-20 shrink-0">
                    <span className="text-sm font-semibold text-violet-400">Translated</span>
                    <a href={translatedUrl} download={translatedFileName} className="inline-flex items-center gap-1 bg-[#8B5CF6] hover:bg-[#7C3AED] text-white rounded px-3 py-1.5 text-xs font-bold transition-colors">
                      <span className="material-symbols-outlined text-[16px]">download</span> Download
                    </a>
                  </div>
                  <div className="flex-1 overflow-hidden relative">
                    <DocumentPreview file={translatedFile} publicUrl={translatedPublicUrl} />
                  </div>
                </div>
              )}
            </div>
          </div>
        )}

        {/* Progress Overlay */}
        {isTranslating && (
          <div className="absolute inset-0 bg-black/80 backdrop-blur-sm z-50 flex flex-col items-center justify-center">
            <div className="relative w-24 h-24 flex items-center justify-center mb-6">
              <div className="absolute inset-0 rounded-full border border-[#8B5CF6] animate-ping opacity-50"></div>
              <span className="text-4xl font-bold bg-gradient-to-b from-white to-[#8B5CF6] bg-clip-text text-transparent">अ</span>
            </div>
            <h2 className="text-2xl font-bold text-white mb-2">Translating...</h2>
            <div className="w-64 bg-white/10 h-1.5 rounded-full mt-4 overflow-hidden">
              <div className="bg-[#8B5CF6] h-full transition-all duration-300 w-1/2 animate-pulse"></div>
            </div>
          </div>
        )}

      </main>
    </div>
  );
}

export default App;
