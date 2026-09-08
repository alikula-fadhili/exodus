// Project Exodus Mission Control — Native Desktop Logic

// Tauri v2 invoke helper with REST API fallback
const invoke = async (cmd, args = {}) => {
  if (window.__TAURI__?.core?.invoke) {
    return await window.__TAURI__.core.invoke(cmd, args);
  }
  if (window.__TAURI__?.invoke) {
    return await window.__TAURI__.invoke(cmd, args);
  }
  if (window.__TAURI_INTERNALS__?.invoke) {
    return await window.__TAURI_INTERNALS__.invoke(cmd, args);
  }
  
  try {
    switch (cmd) {
      case 'list_operations':
      case 'get_operational_items': {
        const res = await fetch('/api/operations');
        return await res.json();
      }
      case 'get_operation': {
        const res = await fetch(`/api/operations/${args.id}`);
        return await res.json();
      }
      case 'ingest_operation': {
        const res = await fetch('/api/operations/ingest', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(args),
        });
        return await res.json();
      }
      case 'verify_operation': {
        const res = await fetch(`/api/operations/${args.id}/verify`, { method: 'POST' });
        return await res.json();
      }
      case 'approve_operation': {
        const res = await fetch(`/api/operations/${args.id}/approve`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ approver: args.approver || 'operator', notes: args.notes || '' }),
        });
        return await res.json();
      }
      case 'reject_operation': {
        const res = await fetch(`/api/operations/${args.id}/reject`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ actor: args.actor || 'operator', reason: args.reason || '' }),
        });
        return await res.json();
      }
      case 'advance_card_stage': {
        const res = await fetch(`/api/operations/${args.id}/advance`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ target_stage: args.target_stage }),
        });
        return await res.json();
      }
      case 'scan_local_repository': {
        const res = await fetch('/api/project/scan', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(args),
        });
        return await res.json();
      }
      case 'setup_project_workflow': {
        const res = await fetch('/api/project/setup', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(args),
        });
        return await res.json();
      }
      case 'get_project_structure': {
        const res = await fetch('/api/project/structure');
        return await res.json();
      }
      case 'get_agent_api_keys': {
        const res = await fetch('/api/settings/keys');
        return await res.json();
      }
      case 'save_agent_api_keys': {
        const res = await fetch('/api/settings/keys', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(args.keys),
        });
        return await res.json();
      }
      case 'get_esg_topology': {
        const res = await fetch('/api/graph');
        return await res.json();
      }
      default:
        console.warn(`[Unknown invoke command] ${cmd}`, args);
        return null;
    }
  } catch (err) {
    console.error(`[Invoke Error] ${cmd}:`, err);
  }
};

let currentItems = [];
let selectedItemId = null;
let currentFilter = 'all';
let boardFilter = 'all';
let boardSearchQuery = '';
let isAutoPipelineRunning = false;
let recentlyMovedId = null;
let activeTab = 'board';

// ============================================================================
// Navigation & Tab Switching
// ============================================================================
function initNavigation() {
  const tabs = [
    { id: 'tab-btn-board', view: 'view-board', name: 'board' },
    { id: 'tab-btn-structure', view: 'view-structure', name: 'structure' },
    { id: 'tab-btn-queue', view: 'view-queue', name: 'queue' },
    { id: 'tab-btn-graph', view: 'view-graph', name: 'graph' },
  ];

  tabs.forEach(t => {
    const btn = document.getElementById(t.id);
    if (!btn) return;
    btn.addEventListener('click', () => {
      tabs.forEach(other => {
        document.getElementById(other.id)?.classList.remove('active');
        document.getElementById(other.view)?.classList.add('hidden');
      });
      btn.classList.add('active');
      document.getElementById(t.view)?.classList.remove('hidden');
      activeTab = t.name;

      if (t.name === 'structure') {
        loadProjectStructure();
      } else if (t.name === 'graph') {
        loadChangeGraph();
      } else if (t.name === 'board') {
        renderBoard();
      } else if (t.name === 'queue') {
        renderQueueList();
      }
    });
  });
}

// ============================================================================
// Board View Logic (Linear / Notion Style Kanban)
// ============================================================================
const DEFAULT_STAGES = [
  { id: 'stage-backlog', label: 'Backlog', relation: 'captured', dot: 'dot-blue', wipLimit: 0, visible: true },
  { id: 'stage-analyzing', label: 'In Sandbox', relation: 'sandboxed', dot: 'dot-amber', wipLimit: 4, visible: true },
  { id: 'stage-testing', label: 'Verified', relation: 'verified', dot: 'dot-cyan', wipLimit: 3, visible: true },
  { id: 'stage-approved', label: 'Approved', relation: 'approved', dot: 'dot-emerald', wipLimit: 0, visible: true },
  { id: 'stage-done', label: 'In Master', relation: 'promoted', dot: 'dot-purple', wipLimit: 0, visible: true },
];

function getBoardConfig() {
  try {
    const saved = localStorage.getItem('exodus_board_config_v2');
    if (saved) return JSON.parse(saved);
  } catch (_) {}
  return DEFAULT_STAGES;
}

function saveBoardConfig(config) {
  localStorage.setItem('exodus_board_config_v2', JSON.stringify(config));
}

function renderBoard() {
  const boardGrid = document.getElementById('board-grid');
  if (!boardGrid) return;

  const config = getBoardConfig();
  const visibleStages = config.filter(s => s.visible);
  const query = (boardSearchQuery || '').toLowerCase().trim();

  const filtered = currentItems.filter(i => {
    // Tag / Domain filter
    if (boardFilter !== 'all') {
      const itemTags = i.tags || [i.domain_tag || '#prod-bug'];
      const matches = itemTags.includes(boardFilter) || i.domain_tag === boardFilter;
      if (!matches) return false;
    }
    // Search query filter
    if (query) {
      const titleMatch = (i.title || '').toLowerCase().includes(query);
      const descMatch = (i.description || '').toLowerCase().includes(query);
      const idMatch = (i.id || '').toLowerCase().includes(query);
      const actorMatch = (i.requester || '').toLowerCase().includes(query);
      return titleMatch || descMatch || idMatch || actorMatch;
    }
    return true;
  });

  const buckets = {};
  visibleStages.forEach(s => { buckets[s.id] = []; });

  filtered.forEach(item => {
    const s = (item.state || '').toLowerCase();
    let targetStage = visibleStages.find(stage => {
      if (stage.relation === 'captured' && (s === 'captured' || s === '')) return true;
      if (stage.relation === 'sandboxed' && s === 'sandboxed') return true;
      if (stage.relation === 'verified' && (s === 'contract_verified' || s === 'contractverified' || s === 'degraded')) return true;
      if (stage.relation === 'approved' && (s === 'human_approved' || s === 'humanapproved')) return true;
      if (stage.relation === 'promoted' && s === 'promoted') return true;
      return false;
    });

    if (!targetStage && visibleStages.length > 0) {
      targetStage = visibleStages[0];
    }
    if (targetStage && buckets[targetStage.id]) {
      buckets[targetStage.id].push(item);
    }
  });

  boardGrid.style.gridTemplateColumns = `repeat(${visibleStages.length}, minmax(280px, 1fr))`;
  boardGrid.innerHTML = visibleStages.map(stage => {
    const itemsInStage = buckets[stage.id] || [];
    const count = itemsInStage.length;
    const isWipExceeded = stage.wipLimit > 0 && count > stage.wipLimit;

    return `
      <div class="kanban-col" data-stage-id="${stage.id}" data-relation="${stage.relation}" id="col-${stage.id}">
        <div class="col-header">
          <div class="col-title-group">
            <span class="col-dot ${stage.dot || 'dot-blue'}"></span>
            <span class="col-title">${escapeHtml(stage.label)}</span>
            <span class="col-count">${count}</span>
            ${isWipExceeded ? `<span class="text-[9px] font-bold text-rose-400 bg-rose-500/10 px-1.5 py-0.5 rounded border border-rose-500/20">WIP ${count}/${stage.wipLimit}</span>` : ''}
          </div>
          <button class="text-zinc-400 hover:text-white text-sm px-1.5 py-0.5 rounded hover:bg-zinc-800" onclick="window.openNewTaskModal('${stage.relation}')" title="Create task in this stage">+</button>
        </div>

        <div class="col-cards" id="cards-${stage.id}" ondragover="window.handleDragOver(event)" ondrop="window.handleDrop('${stage.relation}', event)">
          ${itemsInStage.length === 0
            ? '<div class="text-center py-6 text-xs text-zinc-600 italic">No tasks in this stage</div>'
            : itemsInStage.map(item => renderBoardCard(item, stage.relation)).join('')}
        </div>
      </div>
    `;
  }).join('');
}

// Render Individual Board Card (No Emojis, Clean Sizing)
function renderBoardCard(item, stageRelation) {
  const isJustMoved = recentlyMovedId === item.id;
  const shortKey = item.id.length > 8 ? item.id.substring(item.id.length - 6).toUpperCase() : item.id.toUpperCase();
  const tags = item.tags || [item.domain_tag || '#prod-bug'];
  const promptPreview = item.prompt ? item.prompt.trim() : '';

  let actionBtn = '';
  if (stageRelation === 'captured') {
    actionBtn = `<button class="card-action-btn" onclick="window.advanceTask('${item.id}', 'verified', event)">Verify</button>`;
  } else if (stageRelation === 'sandboxed') {
    actionBtn = `<button class="card-action-btn" onclick="window.advanceTask('${item.id}', 'verified', event)">Verify</button>`;
  } else if (stageRelation === 'verified') {
    actionBtn = `<button class="card-action-btn btn-promote" onclick="window.advanceTask('${item.id}', 'approved', event)">Approve</button>`;
  } else if (stageRelation === 'approved') {
    actionBtn = `<button class="card-action-btn btn-promote" onclick="window.advanceTask('${item.id}', 'promoted', event)">Apply</button>`;
  } else if (stageRelation === 'promoted') {
    actionBtn = `<span class="text-[10px] font-bold text-emerald-400">In master</span>`;
  }

  return `
    <div class="kanban-card ${isJustMoved ? 'just-moved' : ''}" 
         draggable="true" 
         data-id="${item.id}"
         ondragstart="window.handleDragStart('${item.id}', event)"
         onclick="window.openCardDetail('${item.id}')">
      <div class="card-top">
        <span class="card-key">${shortKey}</span>
        <div class="card-tags">
          ${tags.map(t => `<span class="tag-chip">${escapeHtml(t)}</span>`).join('')}
        </div>
      </div>

      <div class="card-title">${escapeHtml(item.title)}</div>
      <div class="card-desc">${escapeHtml(item.description)}</div>

      ${promptPreview ? `
        <div class="card-prompt-badge" title="${escapeHtml(promptPreview)}">
          <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path></svg>
          <span class="truncate">${escapeHtml(promptPreview)}</span>
        </div>
      ` : ''}

      <div class="card-footer">
        <div class="card-actor">${escapeHtml(item.requester)}</div>
        <div class="card-actions">
          ${actionBtn}
        </div>
      </div>
    </div>
  `;
}

// Stage Advancement Handler
window.advanceTask = async (itemId, targetStage, e) => {
  if (e) e.stopPropagation();
  try {
    recentlyMovedId = itemId;
    const updated = await invoke('advance_card_stage', { id: itemId, targetStage: targetStage });
    
    // Update local list
    const idx = currentItems.findIndex(i => i.id === itemId);
    if (idx !== -1 && updated) {
      currentItems[idx] = updated;
    }
    renderBoard();
    showToast(`Task advanced to ${targetStage}`);
    setTimeout(() => { recentlyMovedId = null; }, 1200);
  } catch (err) {
    showToast(`Error advancing task: ${err}`, 'error');
  }
};

// Drag and Drop Handlers
let draggedItemId = null;

window.handleDragStart = (itemId, e) => {
  draggedItemId = itemId;
  e.dataTransfer.setData('text/plain', itemId);
  e.dataTransfer.effectAllowed = 'move';
};

window.handleDragOver = (e) => {
  e.preventDefault();
  e.dataTransfer.dropEffect = 'move';
};

window.handleDrop = async (targetRelation, e) => {
  e.preventDefault();
  if (!draggedItemId) return;
  const itemId = draggedItemId;
  draggedItemId = null;
  await window.advanceTask(itemId, targetRelation);
};

// Open Card Details in Tasks & Review view
window.openCardDetail = (itemId) => {
  selectedItemId = itemId;
  document.getElementById('tab-btn-queue')?.click();
  selectItemInQueue(itemId);
};

// ============================================================================
// Project Structure & Human Organization Logic
// ============================================================================
async function loadProjectStructure() {
  const container = document.getElementById('struct-domains-container');
  if (!container) return;

  try {
    const structure = await invoke('get_project_structure');
    if (!structure) {
      container.innerHTML = '<div class="p-8 text-center text-zinc-500">No project structure loaded.</div>';
      return;
    }

    document.getElementById('struct-project-name').textContent = structure.project_name || 'Active Project';
    document.getElementById('struct-project-path').textContent = structure.root_path || '.';
    document.getElementById('struct-progress-label').textContent = `${Math.round(structure.overall_progress_pct || 0)}%`;
    document.getElementById('struct-progress-bar').style.width = `${Math.round(structure.overall_progress_pct || 0)}%`;

    const domains = structure.domains || [];
    container.innerHTML = domains.map((domain, dIdx) => `
      <div class="rounded-xl border border-zinc-800 bg-zinc-900/40 overflow-hidden">
        <div class="p-4 bg-zinc-900/70 border-b border-zinc-800 flex justify-between items-center">
          <div>
            <h2 class="text-sm font-bold text-zinc-100">${escapeHtml(domain.name)}</h2>
            <p class="text-xs text-zinc-400 mt-0.5">${escapeHtml(domain.description)}</p>
          </div>
          <span class="px-2.5 py-1 rounded-md text-xs font-semibold bg-zinc-800 text-zinc-300 border border-zinc-700">
            ${domain.verified_units || 0} / ${domain.total_units || 0} Verified
          </span>
        </div>

        <div class="p-4 space-y-4">
          ${(domain.waves || []).map(wave => `
            <div class="rounded-lg border border-zinc-800/80 bg-zinc-950/50 p-4 space-y-3">
              <div class="flex justify-between items-center">
                <div>
                  <h3 class="text-xs font-bold text-platinum">${escapeHtml(wave.wave_name)}</h3>
                  <p class="text-[11px] text-zinc-400">${escapeHtml(wave.description)}</p>
                </div>
                <div class="flex items-center gap-3">
                  <span class="text-xs font-mono text-zinc-400">${wave.verified_units}/${wave.total_units}</span>
                  <div class="w-24 h-1.5 bg-zinc-800 rounded-full overflow-hidden">
                    <div class="h-full bg-emerald-400" style="width: ${wave.completion_pct}%;"></div>
                  </div>
                </div>
              </div>

              <!-- Units List -->
              <div class="grid grid-cols-1 md:grid-cols-2 gap-2 pt-1">
                ${(wave.units || []).map(unit => `
                  <div class="flex justify-between items-center p-2.5 rounded-md border border-zinc-800 bg-zinc-900/60 text-xs hover:border-zinc-700 transition-colors">
                    <div class="space-y-0.5">
                      <div class="font-semibold text-zinc-200">${escapeHtml(unit.symbol_name)}</div>
                      <div class="text-[10px] text-zinc-500 font-mono">${escapeHtml(unit.file_path)}</div>
                    </div>
                    <div class="flex items-center gap-2">
                      <span class="px-2 py-0.5 rounded text-[10px] font-bold ${getUnitStatusClass(unit.status)}">
                        ${escapeHtml(unit.status)}
                      </span>
                      <button class="px-2 py-1 rounded bg-zinc-800 hover:bg-zinc-700 text-[10px] font-medium text-zinc-300" onclick="window.openCardDetail('${unit.unit_id}')">
                        Inspect
                      </button>
                    </div>
                  </div>
                `).join('')}
              </div>
            </div>
          `).join('')}
        </div>
      </div>
    `).join('');
  } catch (err) {
    container.innerHTML = `<div class="p-6 text-center text-rose-400">Failed to load project structure: ${err}</div>`;
  }
}

function getUnitStatusClass(status) {
  const s = (status || '').toLowerCase();
  if (s.includes('verified') || s.includes('promoted') || s.includes('approved')) {
    return 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20';
  }
  if (s.includes('sandboxed') || s.includes('testing')) {
    return 'bg-amber-500/10 text-amber-400 border border-amber-500/20';
  }
  return 'bg-zinc-800 text-zinc-400 border border-zinc-700';
}

// ============================================================================
// Project Discovery & Setup Wizard Workflow
// ============================================================================
function initProjectSetupWizard() {
  const modal = document.getElementById('modal-project-setup');
  const btnOpen = document.getElementById('btn-open-project');
  const btnClose = document.getElementById('btn-close-project-setup');
  const btnCancel = document.getElementById('btn-cancel-project-setup');
  const btnScan = document.getElementById('btn-run-scan');
  const btnBrowse = document.getElementById('btn-browse-folder');
  const htmlFolderPicker = document.getElementById('html-folder-picker');
  const pathInput = document.getElementById('setup-repo-path');
  const scanError = document.getElementById('setup-scan-error');
  const btnStart = document.getElementById('btn-start-project-setup');
  const discoveryCard = document.getElementById('setup-discovery-card');
  const pipeline = document.getElementById('setup-progress-pipeline');

  const resetPipelineSteps = () => {
    const steps = ['pipe-step-1', 'pipe-step-2', 'pipe-step-3', 'pipe-step-4', 'pipe-step-5'];
    steps.forEach((s, idx) => {
      const el = document.getElementById(s);
      if (el) {
        el.classList.remove('border-emerald-500/50', 'bg-emerald-500/5');
        const sp = el.querySelector('span');
        if (sp) {
          sp.textContent = `${idx + 1}`;
          sp.classList.remove('border-emerald-400', 'text-emerald-400');
        }
      }
    });
  };

  btnOpen?.addEventListener('click', () => {
    modal.classList.remove('hidden');
    discoveryCard?.classList.add('hidden');
    pipeline?.classList.add('hidden');
    resetPipelineSteps();
    if (scanError) {
      scanError.classList.add('hidden');
      scanError.textContent = '';
    }
    btnStart.disabled = !(pathInput && pathInput.value.trim().length > 0);
  });

  pathInput?.addEventListener('input', () => {
    btnStart.disabled = !(pathInput.value.trim().length > 0);
  });

  const closeModal = () => modal.classList.add('hidden');
  btnClose?.addEventListener('click', closeModal);
  btnCancel?.addEventListener('click', closeModal);
  modal?.addEventListener('click', (e) => {
    if (e.target === modal) closeModal();
  });

  pathInput?.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      btnScan?.click();
    }
  });

  btnBrowse?.addEventListener('click', async () => {
    try {
      btnBrowse.disabled = true;
      const currentVal = pathInput?.value?.trim() || '';
      const selected = await invoke('pick_folder', { defaultPath: currentVal || null });
      if (selected) {
        if (pathInput) pathInput.value = selected;
        if (scanError) {
          scanError.classList.add('hidden');
          scanError.textContent = '';
        }
        btnScan?.click();
      }
    } catch (err) {
      console.warn('Native folder picker not available, attempting fallback:', err);
      htmlFolderPicker?.click();
    } finally {
      btnBrowse.disabled = false;
    }
  });

  htmlFolderPicker?.addEventListener('change', (e) => {
    const files = e.target.files;
    if (files && files.length > 0) {
      const firstFile = files[0];
      const relPath = firstFile.webkitRelativePath || '';
      const folderName = relPath.split('/')[0];
      if (folderName && pathInput) {
        pathInput.value = folderName;
        btnScan?.click();
      }
    }
  });

  btnScan?.addEventListener('click', async () => {
    const path = pathInput?.value?.trim();
    if (!path) {
      showToast('Please enter or select a repository directory.', 'error');
      return;
    }

    btnScan.disabled = true;
    btnScan.textContent = 'Scanning...';
    if (scanError) {
      scanError.classList.add('hidden');
      scanError.textContent = '';
    }

    try {
      const result = await invoke('scan_local_repository', { path });
      if (result) {
        document.getElementById('discover-name').textContent = result.name;
        document.getElementById('discover-lang').textContent = result.detected_language;
        document.getElementById('discover-manifests').textContent = result.manifest_files.join(', ') || 'None';
        document.getElementById('discover-files').textContent = result.total_files;
        document.getElementById('discover-lines').textContent = result.total_lines_approx.toLocaleString();
        
        if (result.path && pathInput) {
          pathInput.value = result.path;
        }

        discoveryCard?.classList.remove('hidden');
        btnStart.disabled = false;
        showToast(`Repository scanned: detected ${result.detected_language}`);
      }
    } catch (err) {
      const msg = typeof err === 'string' ? err : (err?.message || JSON.stringify(err));
      if (scanError) {
        scanError.textContent = `Scan failed: ${msg}. Try using the Browse button to select an existing directory.`;
        scanError.classList.remove('hidden');
      }
      showToast(`Scan failed: ${msg}`, 'error');
    } finally {
      btnScan.disabled = false;
      btnScan.textContent = 'Scan Repo';
    }
  });

  btnStart?.addEventListener('click', async () => {
    const path = document.getElementById('setup-repo-path')?.value.trim();
    const targetLang = document.getElementById('setup-target-lang')?.value || 'Rust 2021';
    if (!path) return;

    btnStart.disabled = true;
    pipeline?.classList.remove('hidden');

    // Animate process-by-process steps
    const steps = ['pipe-step-1', 'pipe-step-2', 'pipe-step-3', 'pipe-step-4', 'pipe-step-5'];
    for (let i = 0; i < steps.length; i++) {
      const el = document.getElementById(steps[i]);
      if (el) {
        el.classList.add('border-emerald-500/50', 'bg-emerald-500/5');
        el.querySelector('span').textContent = '✓';
        el.querySelector('span').classList.add('border-emerald-400', 'text-emerald-400');
      }
      await new Promise(r => setTimeout(r, 200));
    }

    try {
      const result = await invoke('setup_project_workflow', { path, targetLang });
      showToast(result.message || 'Project initialized successfully!');
      document.getElementById('active-repo-badge').textContent = (result.project_name || 'PROJECT').toUpperCase();
      
      closeModal();
      await loadOperations();
      renderBoard();
      if (activeTab === 'structure') loadProjectStructure();
      if (activeTab === 'graph') loadChangeGraph();
    } catch (err) {
      showToast(`Setup error: ${err}`, 'error');
    } finally {
      btnStart.disabled = false;
    }
  });
}

// ============================================================================
// Change Lineage Graph Canvas (Dynamic Real AST Semantic Graph)
// ============================================================================
let graphData = { nodes: [], edges: [] };
let selectedGraphNode = null;

async function loadChangeGraph() {
  try {
    const payload = await invoke('get_esg_topology');
    if (payload) {
      graphData = payload;
      document.getElementById('graph-node-count').textContent = payload.nodes.length;
      document.getElementById('graph-edge-count').textContent = payload.edges.length;
      drawGraphCanvas();
    }
  } catch (err) {
    console.error('Failed to load ESG topology:', err);
  }
}

function drawGraphCanvas() {
  const canvas = document.getElementById('esg-canvas');
  if (!canvas) return;
  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  const w = canvas.width;
  const h = canvas.height;
  ctx.clearRect(0, 0, w, h);

  const nodes = graphData.nodes || [];
  const edges = graphData.edges || [];

  if (nodes.length === 0) {
    ctx.fillStyle = '#71717a';
    ctx.font = '13px system-ui';
    ctx.textAlign = 'center';
    ctx.fillText('No graph nodes loaded. Open a project to parse its semantic graph.', w / 2, h / 2);
    return;
  }

  // Position nodes by wave layers
  const waveMap = {};
  nodes.forEach(n => {
    const wave = n.wave || 0;
    if (!waveMap[wave]) waveMap[wave] = [];
    waveMap[wave].push(n);
  });

  const waves = Object.keys(waveMap).map(Number).sort((a, b) => a - b);
  const xSpacing = w / (waves.length + 1);

  const nodePositions = {};

  waves.forEach((wave, wIdx) => {
    const layerNodes = waveMap[wave];
    const ySpacing = h / (layerNodes.length + 1);
    layerNodes.forEach((node, nIdx) => {
      nodePositions[node.id] = {
        x: xSpacing * (wIdx + 1),
        y: ySpacing * (nIdx + 1),
        node,
      };
    });
  });

  // Draw Edges
  edges.forEach(edge => {
    const from = nodePositions[edge.from];
    const to = nodePositions[edge.to];
    if (from && to) {
      ctx.beginPath();
      ctx.moveTo(from.x, from.y);
      ctx.lineTo(to.x, to.y);
      ctx.strokeStyle = edge.is_cycle_edge ? '#f87171' : 'rgba(255, 255, 255, 0.15)';
      ctx.lineWidth = edge.is_cycle_edge ? 2 : 1;
      ctx.stroke();
    }
  });

  // Draw Nodes
  Object.values(nodePositions).forEach(item => {
    const { x, y, node } = item;
    const isSelected = selectedGraphNode?.id === node.id;

    ctx.beginPath();
    ctx.arc(x, y, isSelected ? 14 : 10, 0, Math.PI * 2);
    
    // Fill color by kind
    if (node.kind === 'module') ctx.fillStyle = '#60a5fa';
    else if (node.kind === 'class') ctx.fillStyle = '#c084fc';
    else if (node.kind === 'symbol') ctx.fillStyle = '#38bdf8';
    else if (node.kind === 'contract' || node.kind === 'test') ctx.fillStyle = '#34d399';
    else ctx.fillStyle = '#ffffff';

    ctx.fill();

    if (isSelected) {
      ctx.strokeStyle = '#ffffff';
      ctx.lineWidth = 2;
      ctx.stroke();
    }

    // Label
    ctx.fillStyle = isSelected ? '#ffffff' : '#d4d4d8';
    ctx.font = '10px system-ui';
    ctx.textAlign = 'center';
    ctx.fillText(node.label, x, y + 22);
  });

  // Click handler on canvas
  canvas.onclick = (e) => {
    const rect = canvas.getBoundingClientRect();
    const clickX = (e.clientX - rect.left) * (canvas.width / rect.width);
    const clickY = (e.clientY - rect.top) * (canvas.height / rect.height);

    for (const item of Object.values(nodePositions)) {
      const dist = Math.hypot(item.x - clickX, item.y - clickY);
      if (dist <= 16) {
        selectedGraphNode = item.node;
        inspectGraphNode(item.node);
        drawGraphCanvas();
        return;
      }
    }
  };
}

function inspectGraphNode(node) {
  document.getElementById('drawer-node-kind').textContent = node.kind.toUpperCase();
  document.getElementById('drawer-node-label').textContent = node.label;
  document.getElementById('drawer-node-sub').textContent = node.subtitle || '';
  document.getElementById('drawer-node-detail').textContent = node.detail || 'No detailed diagnostics available.';

  const diffSection = document.getElementById('drawer-diff-section');
  const diffContent = document.getElementById('drawer-diff-content');
  if (node.diff_snippet) {
    diffSection.classList.remove('hidden');
    diffContent.textContent = node.diff_snippet;
  } else {
    diffSection.classList.add('hidden');
  }

  const metaSection = document.getElementById('drawer-meta-section');
  const metaGrid = document.getElementById('drawer-meta-grid');
  if (node.meta && Object.keys(node.meta).length > 0) {
    metaSection.classList.remove('hidden');
    metaGrid.innerHTML = Object.entries(node.meta).map(([k, v]) => `
      <div class="text-xs">
        <span class="text-zinc-500 block uppercase font-mono text-[9px]">${escapeHtml(k)}</span>
        <strong class="text-zinc-200">${escapeHtml(v)}</strong>
      </div>
    `).join('');
  } else {
    metaSection.classList.add('hidden');
  }
}

// ============================================================================
// Settings & Agent API Keys Modal
// ============================================================================
function initSettingsModal() {
  const modal = document.getElementById('modal-sdlc-settings');
  const btnOpen = document.getElementById('btn-sdlc-settings');
  const btnClose = document.getElementById('btn-close-sdlc');
  const btnCancel = document.getElementById('btn-cancel-sdlc');
  const btnSave = document.getElementById('btn-save-settings');

  if (!modal) return;

  btnOpen?.addEventListener('click', async () => {
    modal.classList.remove('hidden');
    try {
      const keys = await invoke('get_agent_api_keys');
      if (keys) {
        if (keys.anthropic_api_key) document.getElementById('agent-key-anthropic').value = keys.anthropic_api_key;
        if (keys.openai_api_key) document.getElementById('agent-key-openai').value = keys.openai_api_key;
        if (keys.gemini_api_key) document.getElementById('agent-key-gemini').value = keys.gemini_api_key;
        if (keys.local_endpoint) document.getElementById('agent-endpoint-local').value = keys.local_endpoint;
        if (keys.default_model) document.getElementById('agent-default-model').value = keys.default_model;
      }
    } catch (err) {
      console.error('Failed to load settings:', err);
    }
  });

  const closeModal = () => modal.classList.add('hidden');
  btnClose?.addEventListener('click', closeModal);
  btnCancel?.addEventListener('click', closeModal);
  modal?.addEventListener('click', (e) => {
    if (e.target === modal) closeModal();
  });

  btnSave?.addEventListener('click', async () => {
    const keys = {
      anthropic_api_key: document.getElementById('agent-key-anthropic')?.value.trim() || null,
      openai_api_key: document.getElementById('agent-key-openai')?.value.trim() || null,
      gemini_api_key: document.getElementById('agent-key-gemini')?.value.trim() || null,
      local_endpoint: document.getElementById('agent-endpoint-local')?.value.trim() || null,
      default_model: document.getElementById('agent-default-model')?.value || 'claude-3-5-sonnet',
    };

    try {
      await invoke('save_agent_api_keys', { keys });
      showToast('Settings & Agent API Keys saved successfully');
      closeModal();
    } catch (err) {
      showToast(`Failed to save settings: ${err}`, 'error');
    }
  });
}

// ============================================================================
// Tasks & Review Queue Pane
// ============================================================================
function renderQueueList() {
  const list = document.getElementById('items-list');
  if (!list) return;

  const filtered = currentFilter === 'all'
    ? currentItems
    : currentItems.filter(i => (i.tags || []).includes(currentFilter) || i.domain_tag === currentFilter);

  if (filtered.length === 0) {
    list.innerHTML = '<div class="empty-state">No tasks matching active filter.</div>';
    return;
  }

  list.innerHTML = filtered.map(item => `
    <div class="queue-item ${selectedItemId === item.id ? 'selected' : ''}" onclick="window.selectItemInQueue('${item.id}')">
      <div class="flex justify-between items-center mb-1">
        <span class="text-[10px] font-mono text-zinc-500">${item.id.substring(item.id.length - 6).toUpperCase()}</span>
        <span class="badge ${item.domain_tag === '#prod-bug' ? 'badge-emerald' : 'badge-amber'}">${item.domain_tag || '#prod-bug'}</span>
      </div>
      <div class="text-xs font-bold text-zinc-200 truncate">${escapeHtml(item.title)}</div>
      <div class="text-[11px] text-zinc-400 truncate mt-0.5">${escapeHtml(item.description)}</div>
    </div>
  `).join('');
}

window.selectItemInQueue = async (itemId) => {
  selectedItemId = itemId;
  renderQueueList();

  const emptyReview = document.getElementById('empty-review');
  const activeReview = document.getElementById('active-review');

  try {
    const item = await invoke('get_operation', { id: itemId });
    if (!item) return;

    emptyReview?.classList.add('hidden');
    activeReview?.classList.remove('hidden');

    document.getElementById('review-id').textContent = item.id;
    document.getElementById('review-title').textContent = item.title;
    document.getElementById('review-desc').textContent = item.description;
    document.getElementById('review-state-badge').textContent = item.state;
    document.getElementById('review-domain-badge').textContent = item.domain_tag || '#prod-bug';

    const promptBox = document.getElementById('attached-prompt-text');
    if (promptBox) {
      promptBox.textContent = item.prompt || 'No custom prompt attached.';
    }

    // Update stepper
    const states = ['captured', 'sandboxed', 'verified', 'approved', 'promoted'];
    const curIdx = states.indexOf((item.state || '').toLowerCase());
    states.forEach((s, idx) => {
      const node = document.getElementById(`step-${s}`);
      if (node) {
        node.classList.remove('completed', 'active');
        if (idx < curIdx) node.classList.add('completed');
        else if (idx === curIdx) node.classList.add('active');
      }
    });

    // Wire review action buttons
    document.getElementById('btn-action-verify').onclick = () => window.advanceTask(item.id, 'verified');
    document.getElementById('btn-action-approve').onclick = () => window.advanceTask(item.id, 'approved');
    document.getElementById('btn-action-reject').onclick = () => window.advanceTask(item.id, 'captured');
  } catch (err) {
    showToast(`Error loading item: ${err}`, 'error');
  }
};

// ============================================================================
// Core Ingestion & Operations
// ============================================================================
async function loadOperations() {
  try {
    const items = await invoke('list_operations');
    if (items) {
      currentItems = items;
      renderBoard();
      renderQueueList();
    }
  } catch (err) {
    console.error('Failed to load operations:', err);
  }
}

window.openNewTaskModal = (relation) => {
  const modal = document.getElementById('modal-ingest');
  if (modal) modal.classList.remove('hidden');
};

function initNewTaskModal() {
  const modal = document.getElementById('modal-ingest');
  const btnOpen = document.getElementById('btn-new-item');
  const btnClose = document.getElementById('btn-close-ingest');
  const btnCancel = document.getElementById('btn-cancel-ingest');
  const btnSubmit = document.getElementById('btn-submit-ingest');

  btnOpen?.addEventListener('click', () => modal?.classList.remove('hidden'));
  const closeModal = () => modal?.classList.add('hidden');
  btnClose?.addEventListener('click', closeModal);
  btnCancel?.addEventListener('click', closeModal);
  modal?.addEventListener('click', (e) => {
    if (e.target === modal) closeModal();
  });

  btnSubmit?.addEventListener('click', async () => {
    const title = document.getElementById('new-item-title')?.value.trim();
    const desc = document.getElementById('new-item-desc')?.value.trim();
    const domainTag = document.getElementById('new-item-domain')?.value || '#prod-bug';
    const requester = document.getElementById('new-item-requester')?.value.trim() || 'engineer';

    if (!title) return;

    try {
      await invoke('ingest_operation', {
        title,
        description: desc || null,
        requester,
        domainTag,
        payload: null,
        prompt: desc || null,
        systemGoal: null,
        tags: [domainTag],
      });
      showToast('New task created successfully');
      closeModal();
      await loadOperations();
    } catch (err) {
      showToast(`Creation failed: ${err}`, 'error');
    }
  });
}

// Toast helper
function showToast(msg, type = 'info') {
  const container = document.getElementById('toast-container');
  if (!container) return;
  const toast = document.createElement('div');
  toast.className = `toast ${type === 'error' ? 'border-rose-500/50 text-rose-300' : 'border-zinc-700'}`;
  toast.textContent = msg;
  container.appendChild(toast);
  setTimeout(() => { toast.remove(); }, 3500);
}

function escapeHtml(str) {
  if (!str) return '';
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

// Auto-run pipeline demonstration
function initAutoPipeline() {
  const btn = document.getElementById('btn-auto-pipeline');
  btn?.addEventListener('click', async () => {
    if (isAutoPipelineRunning) return;
    isAutoPipelineRunning = true;
    btn.disabled = true;
    showToast('Starting automated pipeline execution across board units...');

    for (const item of currentItems) {
      if (item.state === 'Captured') {
        await window.advanceTask(item.id, 'verified');
        await new Promise(r => setTimeout(r, 600));
      }
    }

    showToast('Auto-pipeline run completed');
    isAutoPipelineRunning = false;
    btn.disabled = false;
  });
}

// Application Initialization
async function boot() {
  initNavigation();
  initProjectSetupWizard();
  initSettingsModal();
  initNewTaskModal();
  initAutoPipeline();

  // Filter chips
  document.querySelectorAll('.filter-chip').forEach(chip => {
    chip.addEventListener('click', () => {
      document.querySelectorAll('.filter-chip').forEach(c => c.classList.remove('active'));
      chip.classList.add('active');
      boardFilter = chip.getAttribute('data-filter') || 'all';
      renderBoard();
    });
  });

  // Real-time board search query filter
  const searchInput = document.getElementById('board-search-input');
  searchInput?.addEventListener('input', (e) => {
    boardSearchQuery = e.target.value;
    renderBoard();
  });

  await loadOperations();
}

if (document.readyState === 'loading') {
  window.addEventListener('DOMContentLoaded', boot);
} else {
  boot();
}
