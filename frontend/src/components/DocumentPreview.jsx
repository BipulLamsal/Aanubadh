import React, { useEffect, useState, useRef } from 'react';
import Papa from 'papaparse';
import { DocxEditor } from '@eigenpal/docx-js-editor';
import '@eigenpal/docx-js-editor/styles.css';

const isDeployed = window.location.hostname !== 'localhost' && window.location.hostname !== '127.0.0.1';

export default function DocumentPreview({ file }) {
  const [content, setContent] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);

  useEffect(() => {
    if (!file) {
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);
    setContent(null);

    const loadPreview = async () => {
      try {
        if (file.name.endsWith('.docx')) {
          const arrayBuffer = await file.arrayBuffer();
          setContent({ type: 'docx', buffer: arrayBuffer });
        } else if (file.name.endsWith('.csv')) {
          Papa.parse(file, {
            complete: (results) => {
              setContent({ type: 'csv', data: results.data });
            },
            error: () => setError('Failed to parse CSV')
          });
        } else if (file.name.endsWith('.pdf')) {
          const url = URL.createObjectURL(file);
          setContent({ type: 'pdf', url });
        } else {
          const text = await file.text();
          setContent({ type: 'text', text });
        }
      } catch (err) {
        console.error(err);
        setError('Preview not available');
      } finally {
        setLoading(false);
      }
    };

    loadPreview();
  }, [file]);

  if (loading) {
    return <div className="flex items-center justify-center h-full text-zinc-500">Loading preview...</div>;
  }

  if (error) {
    return <div className="text-red-500 p-4">{error}</div>;
  }

  if (!content) {
    return null;
  }

  if (content.type === 'docx') {
    return (
      <div className="w-full h-full bg-white text-black overflow-hidden relative flex flex-col" style={{ padding: 0 }}>
        <div className="bg-amber-50 border-b border-amber-200 px-3 py-1.5 flex items-center gap-2 shrink-0">
          <span className="material-symbols-outlined text-amber-600 text-[16px]">info</span>
          <span className="text-[10px] md:text-xs text-amber-800 font-medium leading-tight">
            Formatting, images, and layout may vary in preview. Download the file for the exact version.
          </span>
        </div>
        <div className="flex-1 relative">
          <div className="absolute inset-0">
            <DocxEditor documentBuffer={content.buffer} />
          </div>
        </div>
      </div>
    );
  }

  if (content.type === 'csv') {
    return (
      <div className="w-full h-full overflow-auto bg-white p-0">
        <table className="w-full text-sm text-left text-gray-800 border-collapse whitespace-nowrap">
          <thead className="text-xs text-gray-700 uppercase bg-gray-100 sticky top-0 shadow-sm z-10">
            <tr>
              {content.data[0]?.map((cell, j) => (
                <th key={j} className="px-6 py-3 border-b border-gray-200">{cell}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {content.data.slice(1).map((row, i) => (
              <tr key={i} className="bg-white border-b hover:bg-gray-50">
                {row.map((cell, j) => (
                  <td key={j} className="px-6 py-4">{cell}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  }

  if (content.type === 'pdf') {
    return (
      <div className="w-full h-full flex flex-col bg-[#1a1a1a]">
        <iframe 
          src={content.url} 
          width="100%" 
          height="100%" 
          style={{ border: 'none' }} 
          title="PDF Preview"
          className="flex-1"
        ></iframe>
        
        {/* Mobile helper: iframe often fails for local blobs on mobile */}
        <div className="p-3 bg-black/40 border-t border-white/10 flex items-center justify-between">
          <span className="text-xs text-slate-400">Preview not loading? </span>
          <a 
            href={content.url} 
            target="_blank" 
            rel="noopener noreferrer"
            className="text-xs font-bold text-violet-400 hover:text-violet-300 flex items-center gap-1"
          >
            Open in Tab
          </a>
        </div>
      </div>
    );
  }

  if (content.type === 'text') {
    return (
      <div className="doc-preview p-5">
        <pre style={{ whiteSpace: 'pre-wrap', fontFamily: 'monospace' }}>
          {content.text}
        </pre>
      </div>
    );
  }

  return null;
}
