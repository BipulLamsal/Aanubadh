const state = {
  file: null,
  isBusy: false,
};

const sourceLang = document.getElementById("sourceLang");
const targetLang = document.getElementById("targetLang");
const swapBtn = document.getElementById("swapBtn");
const dropzone = document.getElementById("dropzone");
const fileInput = document.getElementById("fileInput");
const fileCard = document.getElementById("fileCard");
const fileName = document.getElementById("fileName");
const fileSize = document.getElementById("fileSize");
const removeFileBtn = document.getElementById("removeFileBtn");
const translateBtn = document.getElementById("translateBtn");
const status = document.getElementById("status");

const progressSection = document.getElementById("progressSection");
const progressStatus = document.getElementById("progressStatus");
const progressPercent = document.getElementById("progressPercent");
const progressFill = document.getElementById("progressFill");

const downloadSection = document.getElementById("downloadSection");
const downloadBtn = document.getElementById("downloadBtn");

const SUPPORTED_EXTENSIONS = ['.docx', '.pdf', '.csv', '.tsv'];

function readableSize(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

function updateUI() {
  if (state.file) {
    dropzone.classList.add("hidden");
    fileCard.classList.remove("hidden");
    fileName.textContent = state.file.name;
    fileSize.textContent = readableSize(state.file.size);
    translateBtn.disabled = state.isBusy;
    translateBtn.classList.remove("hidden");
  } else {
    dropzone.classList.remove("hidden");
    fileCard.classList.add("hidden");
    translateBtn.disabled = true;
    translateBtn.classList.remove("hidden");
    progressSection.classList.add("hidden");
    downloadSection.classList.add("hidden");
    status.textContent = "";
  }
}

function setBusy(busy) {
  state.isBusy = busy;
  translateBtn.disabled = busy || !state.file;
  removeFileBtn.disabled = busy;
  if (busy) {
      translateBtn.classList.add("hidden");
      progressSection.classList.remove("hidden");
  } else {
      translateBtn.classList.remove("hidden");
  }
}

function showError(message) {
  status.textContent = message;
  status.style.color = "#ba1a1a";
}

function showInfo(message) {
  status.textContent = message;
  status.style.color = "";
}

function handleSelectedFile(file) {
  if (!file) return;

  const ext = file.name.substring(file.name.lastIndexOf('.')).toLowerCase();
  if (!SUPPORTED_EXTENSIONS.includes(ext)) {
    state.file = null;
    updateUI();
    showError(`Unsupported file. Please upload ${SUPPORTED_EXTENSIONS.join(', ')}.`);
    return;
  }

  state.file = file;
  updateUI();
  showInfo("File ready to translate.");
}

async function pollProgress(jobId) {
    while (true) {
        try {
            const res = await fetch(`/api/progress/${jobId}`);
            if (!res.ok) throw new Error("Failed to fetch progress");
            const data = await res.json();
            
            progressFill.style.width = `${data.progress}%`;
            progressPercent.textContent = `${data.progress}%`;

            if (data.status === "done") {
                progressStatus.textContent = "Translation Complete!";
                return true;
            } else if (data.status === "error") {
                throw new Error(data.error || "Translation failed on server.");
            }
        } catch (e) {
            throw e;
        }
        await new Promise(r => setTimeout(r, 500));
    }
}

async function translateNow() {
  if (!state.file || state.isBusy) return;

  const src = sourceLang.value;
  const tgt = targetLang.value;

  if (src === tgt) {
    showError("Source and target languages must be different.");
    return;
  }

  try {
    setBusy(true);
    showInfo("Uploading file...");
    progressFill.style.width = "0%";
    progressPercent.textContent = "0%";
    progressStatus.textContent = "Uploading & Translating...";
    downloadSection.classList.add("hidden");

    const formData = new FormData();
    formData.append("file", state.file);
    formData.append("src_lang", src);
    formData.append("tgt_lang", tgt);

    const res = await fetch("/api/translate", {
        method: "POST",
        body: formData,
    });

    if (!res.ok) {
        const errText = await res.text();
        throw new Error(`Upload failed: ${errText}`);
    }

    const { job_id } = await res.json();
    
    await pollProgress(job_id);

    // Done
    showInfo("");
    progressSection.classList.add("hidden");
    downloadSection.classList.remove("hidden");
    downloadBtn.href = `/api/download/${job_id}`;
    
    state.isBusy = false;
    removeFileBtn.disabled = false;
    translateBtn.classList.add("hidden");
    
  } catch (error) {
    showError(error.message || "An error occurred.");
    setBusy(false);
    progressSection.classList.add("hidden");
  }
}

function updateLanguageOptions() {
    const src = sourceLang.value;
    for (let opt of targetLang.options) {
        if (opt.value === src) {
            opt.disabled = true;
            if (targetLang.value === src) {
                targetLang.value = Array.from(targetLang.options).find(o => o.value !== src).value;
            }
        } else {
            opt.disabled = false;
        }
    }
}

function attachHandlers() {
  const openFilePicker = () => { if(!state.isBusy) fileInput.click(); };

  dropzone.addEventListener("click", openFilePicker);
  dropzone.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      openFilePicker();
    }
  });

  fileInput.addEventListener("change", (e) => {
    const [file] = e.target.files || [];
    if (file) handleSelectedFile(file);
    e.target.value = "";
  });

  ["dragenter", "dragover"].forEach((eventName) => {
    dropzone.addEventListener(eventName, (e) => {
      e.preventDefault();
      if(!state.isBusy) dropzone.classList.add("dragging");
    });
  });

  ["dragleave", "drop"].forEach((eventName) => {
    dropzone.addEventListener(eventName, (e) => {
      e.preventDefault();
      dropzone.classList.remove("dragging");
    });
  });

  dropzone.addEventListener("drop", (e) => {
    if(state.isBusy) return;
    const [file] = e.dataTransfer?.files || [];
    handleSelectedFile(file);
  });

  swapBtn.addEventListener("click", () => {
    const src = sourceLang.value;
    sourceLang.value = targetLang.value;
    targetLang.value = src;
    updateLanguageOptions();
  });

  sourceLang.addEventListener("change", updateLanguageOptions);

  removeFileBtn.addEventListener("click", () => {
      state.file = null;
      updateUI();
  });

  translateBtn.addEventListener("click", translateNow);
}

attachHandlers();
updateLanguageOptions();
updateUI();
