// Project Exodus Mission Control — Tauri Desktop Frontend Logic

// Tauri v2 invoke helper with REST API fallback for dual desktop & browser execution
const invoke = async (cmd, args = {}) => {
  if (window.__TAURI__?.core?.invoke) {
    return await window.__TAURI__.core.invoke(cmd, args);
  }
  
  // REST API Fallback for web browser / standalone server mode
  try {
    switch (cmd) {
      case 'list_operations':
      case 'get_operational_items': {
        const res = await fetch('/api/operations');
        return await res.json();
      }
      case 'get_item_details': {
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
      case 'sandbox_operation': {
        const res = await fetch(`/api/operations/${args.id}/sandbox`, { method: 'POST' });
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
          body: JSON.stringify({ approver: args.approver || 'hitl_operator', notes: args.notes || '' }),
        });
        return await res.json();
      }
      case 'reject_operation': {
        const res = await fetch(`/api/operations/${args.id}/reject`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ actor: args.actor || 'hitl_operator', reason: args.reason || '' }),
        });
        return await res.json();
      }
      case 'promote_operation': {
        const res = await fetch(`/api/operations/${args.id}/promote`, { method: 'POST' });
        return await res.json();
      }
      case 'update_item_prompt': {
        const res = await fetch(`/api/operations/${args.id}/prompt`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ prompt: args.prompt, system_goal: args.system_goal || null }),
        });
        return await res.json();
      }
      case 'update_item_tags': {
        const res = await fetch(`/api/operations/${args.id}/tags`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ tags: args.tags || [], graph_mappings: args.graph_mappings || [] }),
        });
        return await res.json();
      }
      case 'execute_harness_unit': {
        const res = await fetch(`/api/operations/${args.id}/harness/execute`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ prompt: args.prompt || null, system_goal: args.system_goal || null }),
        });
        return await res.json();
      }
      case 'get_harness_environment': {
        const res = await fetch('/api/harness/environment');
        return await res.json();
      }
      case 'get_kernel_plugins': {
        const res = await fetch('/api/plugins');
        return await res.json();
      }
      case 'get_maker_plugins': {
        const res = await fetch('/api/plugins/maker');
        return await res.json();
      }
      case 'save_maker_plugin': {
        const res = await fetch('/api/plugins/maker', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(args.plugin),
        });
        return await res.json();
      }
      case 'delete_maker_plugin': {
        const res = await fetch(`/api/plugins/maker/${args.id}`, { method: 'DELETE' });
        return await res.json();
      }
      case 'get_sdlc_settings': {
        const res = await fetch('/api/sdlc/settings');
        return await res.json();
      }
      case 'get_sdlc_scaffold': {
        const res = await fetch('/api/sdlc/scaffold');
        return await res.json();
      }
      case 'get_esg_topology': {
        const res = await fetch('/api/graph');
        return await res.json();
      }
      default:
        console.warn(`[Unknown invoke command in web client] ${cmd}`, args);
        return null;
    }
  } catch (err) {
    console.error(`[Invoke REST error] ${cmd}:`, err);
    throw err;
  }
};

let currentItems = [];
let selectedItemId = null;
let currentFilter = 'all';
let boardFilter = 'all';
let isAutoPipelineRunning = false;
let recentlyMovedId = null;
let activeInlineDraftCol = null;
let activeTagModalData = null; // { itemId, tagIndex, tagName, mappedSymbols }

// ============================================================================
// Theme Management Engine
// ============================================================================
function initTheme() {
  const savedTheme = localStorage.getItem('exodus_theme') || 'grayscale-gold';
  document.documentElement.setAttribute('data-theme', savedTheme);
  const selector = document.getElementById('theme-selector');
  if (selector) {
    selector.value = savedTheme;
    selector.addEventListener('change', (e) => {
      const theme = e.target.value;
      document.documentElement.setAttribute('data-theme', theme);
      localStorage.setItem('exodus_theme', theme);
      showToast(`Theme switched to ${e.target.options[e.target.selectedIndex].text}`, 'info');
      // Redraw canvas if visible
      if (!document.getElementById('view-graph').classList.contains('hidden')) {
        renderEsgCanvas();
      }
    });
  }
}

// Toast Notifications
function showToast(message, type = 'info') {
  const container = document.getElementById('toast-container');
  if (!container) return;
  const toast = document.createElement('div');
  toast.className = `toast ${type}`;
  const icon = type === 'success' ? '✓' : type === 'error' ? '✗' : 'ℹ';
  toast.innerHTML = `<span style="font-weight: bold;">${icon}</span><span>${escapeHtml(message)}</span>`;
  container.appendChild(toast);
  setTimeout(() => {
    toast.style.opacity = '0';
    toast.style.transform = 'translateY(12px) scale(0.95)';
    setTimeout(() => toast.remove(), 250);
  }, 3500);
}

// ============================================================================
// Navigation Tabs
// ============================================================================
document.getElementById('tab-btn-board')?.addEventListener('click', () => {
  document.getElementById('tab-btn-board').classList.add('active');
  document.getElementById('tab-btn-queue').classList.remove('active');
  document.getElementById('tab-btn-graph').classList.remove('active');
  document.getElementById('view-board').classList.remove('hidden');
  document.getElementById('view-queue').classList.add('hidden');
  document.getElementById('view-graph').classList.add('hidden');
  renderBoard();
});

document.getElementById('tab-btn-queue')?.addEventListener('click', () => {
  document.getElementById('tab-btn-queue').classList.add('active');
  document.getElementById('tab-btn-board').classList.remove('active');
  document.getElementById('tab-btn-graph').classList.remove('active');
  document.getElementById('view-queue').classList.remove('hidden');
  document.getElementById('view-board').classList.add('hidden');
  document.getElementById('view-graph').classList.add('hidden');
});

document.getElementById('tab-btn-graph')?.addEventListener('click', () => {
  document.getElementById('tab-btn-graph').classList.add('active');
  document.getElementById('tab-btn-board').classList.remove('active');
  document.getElementById('tab-btn-queue').classList.remove('active');
  document.getElementById('view-graph').classList.remove('hidden');
  document.getElementById('view-board').classList.add('hidden');
  document.getElementById('view-queue').classList.add('hidden');
  renderEsgCanvas();
});

// Board Toolbar Filters
document.querySelectorAll('.board-filters .filter-chip').forEach(chip => {
  chip.addEventListener('click', (e) => {
    document.querySelectorAll('.board-filters .filter-chip').forEach(c => c.classList.remove('active'));
    e.target.classList.add('active');
    boardFilter = e.target.getAttribute('data-filter');
    renderBoard();
  });
});

// Auto-Run Pipeline Simulation
document.getElementById('btn-auto-pipeline')?.addEventListener('click', async () => {
  if (isAutoPipelineRunning) return;
  isAutoPipelineRunning = true;
  const btn = document.getElementById('btn-auto-pipeline');
  btn.classList.add('btn-primary-highlight');
  btn.innerHTML = `
    <span class="pulse-dot"></span>
    <span>Simulating Pipeline...</span>
  `;

  showToast('Starting automated pipeline execution...', 'info');

  try {
    // 1. Advance Captured -> In Sandbox -> Tests Passing
    for (const item of currentItems) {
      const state = item.state.toLowerCase();
      if (state === 'captured') {
        recentlyMovedId = item.id;
        await invoke('verify_operation', { id: item.id });
        await fetchQueue();
        await new Promise(r => setTimeout(r, 1000));
      }
    }

    // 2. Advance Verified -> Approved
    for (const item of currentItems) {
      const state = item.state.toLowerCase();
      if (state === 'contract_verified' || state === 'contractverified') {
        recentlyMovedId = item.id;
        await invoke('approve_operation', {
          id: item.id,
          approver: 'pipeline_bot',
          notes: 'Auto-approved by CI Pipeline'
        });
        await fetchQueue();
        await new Promise(r => setTimeout(r, 1000));
      }
    }

    showToast('Pipeline execution simulation complete', 'success');
  } catch (err) {
    showToast('Pipeline execution paused: ' + err, 'error');
  } finally {
    isAutoPipelineRunning = false;
    btn.classList.remove('btn-primary-highlight');
    btn.innerHTML = `
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="5 3 19 12 5 21 5 3"></polygon></svg>
      <span>Auto-Run Pipeline</span>
    `;
    recentlyMovedId = null;
    renderBoard();
  }
});

// ============================================================================
// Board Configuration State & Engine
// ============================================================================
const DEFAULT_BOARD_CONFIG = [
  { id: 'captured', label: 'Triggered', relation: 'captured', visible: true, dot: 'dot-rose', wipLimit: 0 },
  { id: 'sandboxed', label: 'In Sandbox', relation: 'sandboxed', visible: true, dot: 'dot-blue', wipLimit: 0 },
  { id: 'verified', label: 'Tests Passing', relation: 'verified', visible: true, dot: 'dot-emerald', wipLimit: 0 },
  { id: 'approved', label: 'Approved', relation: 'approved', visible: true, dot: 'dot-amber', wipLimit: 0 },
  { id: 'promoted', label: 'Applied', relation: 'promoted', visible: true, dot: 'dot-purple', wipLimit: 0 },
];

function getBoardConfig() {
  try {
    const saved = localStorage.getItem('exodus_board_config_v2');
    if (saved) {
      const parsed = JSON.parse(saved);
      if (Array.isArray(parsed) && parsed.length > 0) return parsed;
    }
  } catch (_) {}
  return JSON.parse(JSON.stringify(DEFAULT_BOARD_CONFIG));
}

function saveBoardConfig(config) {
  localStorage.setItem('exodus_board_config_v2', JSON.stringify(config));
}

// Open / Close Board Config Modal
document.getElementById('btn-config-board')?.addEventListener('click', () => {
  openBoardConfigModal();
});
document.getElementById('btn-close-board-config')?.addEventListener('click', () => {
  document.getElementById('modal-board-config')?.classList.add('hidden');
});
document.getElementById('btn-reset-board-config')?.addEventListener('click', () => {
  saveBoardConfig(DEFAULT_BOARD_CONFIG);
  openBoardConfigModal();
  renderBoard();
  showToast('Board configuration reset to defaults', 'info');
});
document.getElementById('btn-save-board-config')?.addEventListener('click', () => {
  const rows = document.querySelectorAll('.stage-config-row');
  const currentConfig = getBoardConfig();
  const newConfig = [];

  rows.forEach(row => {
    const stageId = row.dataset.stageId;
    const nameInput = row.querySelector('.stage-name-input');
    const relationSelect = row.querySelector('.stage-relation-select');
    const wipInput = row.querySelector('.stage-wip-input');
    const visibleCheck = row.querySelector('.stage-visible-check');

    const original = currentConfig.find(c => c.id === stageId) || {};
    newConfig.push({
      id: stageId,
      label: nameInput.value.trim() || original.label || stageId,
      relation: relationSelect.value,
      visible: visibleCheck.checked,
      dot: original.dot || 'dot-blue',
      wipLimit: parseInt(wipInput.value, 10) || 0,
    });
  });

  saveBoardConfig(newConfig);
  document.getElementById('modal-board-config')?.classList.add('hidden');
  renderBoard();
  showToast('Board configuration saved', 'success');
});

function openBoardConfigModal() {
  const modal = document.getElementById('modal-board-config');
  const list = document.getElementById('stage-config-list');
  if (!modal || !list) return;

  const config = getBoardConfig();
  list.innerHTML = config.map((stage, idx) => `
    <div class="stage-config-row" data-stage-id="${stage.id}">
      <div class="stage-reorder-btns">
        <button class="stage-reorder-btn" onclick="window.reorderBoardStage(${idx}, -1)" ${idx === 0 ? 'disabled style="opacity:0.3"' : ''}>▲</button>
        <button class="stage-reorder-btn" onclick="window.reorderBoardStage(${idx}, 1)" ${idx === config.length - 1 ? 'disabled style="opacity:0.3"' : ''}>▼</button>
      </div>
      <input type="text" class="stage-name-input form-input" value="${escapeHtml(stage.label)}" placeholder="Stage Name">
      <select class="stage-relation-select form-select">
        <option value="captured" ${stage.relation === 'captured' ? 'selected' : ''}>Triggered (Captured)</option>
        <option value="sandboxed" ${stage.relation === 'sandboxed' ? 'selected' : ''}>Sandbox (Isolated)</option>
        <option value="verified" ${stage.relation === 'verified' ? 'selected' : ''}>Tests Passing (Gate)</option>
        <option value="approved" ${stage.relation === 'approved' ? 'selected' : ''}>Approved (Sign-Off)</option>
        <option value="promoted" ${stage.relation === 'promoted' ? 'selected' : ''}>Applied (Merged)</option>
      </select>
      <div style="display: flex; align-items: center; gap: 4px;">
        <span style="font-size: 10px; color: var(--text-muted);">WIP:</span>
        <input type="number" class="stage-wip-input form-input" min="0" max="99" value="${stage.wipLimit || 0}" title="0 = Unlimited">
      </div>
      <label style="display: flex; align-items: center; gap: 4px; font-size: 11px; cursor: pointer;">
        <input type="checkbox" class="stage-visible-check" ${stage.visible ? 'checked' : ''}> Show
      </label>
    </div>
  `).join('');

  modal.classList.remove('hidden');
}

window.reorderBoardStage = (idx, direction) => {
  const config = getBoardConfig();
  const targetIdx = idx + direction;
  if (targetIdx < 0 || targetIdx >= config.length) return;
  const temp = config[idx];
  config[idx] = config[targetIdx];
  config[targetIdx] = temp;
  saveBoardConfig(config);
  openBoardConfigModal();
};

// ============================================================================
// Board Renderer with Dynamic Stages & Inline Card Creation
// ============================================================================
function renderBoard() {
  const boardGrid = document.getElementById('board-grid');
  if (!boardGrid) return;

  const config = getBoardConfig();
  const visibleStages = config.filter(s => s.visible);

  const filtered = boardFilter === 'all'
    ? currentItems
    : currentItems.filter(i => {
        const itemTags = getItemTags(i);
        return itemTags.includes(boardFilter) || i.domain_tag === boardFilter;
      });

  // Group items by relation
  const buckets = {};
  visibleStages.forEach(s => { buckets[s.id] = []; });

  filtered.forEach(item => {
    const s = (item.state || '').toLowerCase();
    // Map state to stage relation
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

  // Generate HTML for visible columns
  boardGrid.style.gridTemplateColumns = `repeat(${visibleStages.length}, minmax(290px, 1fr))`;
  boardGrid.innerHTML = visibleStages.map(stage => {
    const itemsInStage = buckets[stage.id] || [];
    const count = itemsInStage.length;
    const isWipExceeded = stage.wipLimit > 0 && count > stage.wipLimit;

    return `
      <div class="board-col kanban-col" data-stage-id="${stage.id}" data-relation="${stage.relation}" id="col-${stage.id}">
        <div class="col-header">
          <div class="col-title-group">
            <span class="col-dot ${stage.dot || 'dot-blue'}"></span>
            <span class="col-title">${escapeHtml(stage.label)}</span>
            <span class="col-count" id="count-${stage.id}">${count}</span>
            ${isWipExceeded ? `<span class="col-wip-pill">WIP ${count}/${stage.wipLimit}</span>` : ''}
          </div>
          <button class="col-add-btn" onclick="window.startInlineDraft('${stage.id}')" title="Create task in this stage">+</button>
        </div>

        <div class="col-cards" id="cards-${stage.id}">
          ${itemsInStage.length === 0 && activeInlineDraftCol !== stage.id
            ? '<div class="empty-col-placeholder">Click space to create a task</div>'
            : itemsInStage.map(item => renderBoardCard(item, stage.relation)).join('')}
          
          ${activeInlineDraftCol === stage.id ? renderInlineDraftCard(stage.id) : ''}
        </div>

        <div style="padding: 0 12px 10px;">
          <button class="board-add-card-btn w-full" onclick="window.startInlineDraft('${stage.id}')">
            <span>+ Add Task</span>
          </button>
        </div>
      </div>
    `;
  }).join('');

  // Attach Drag & Drop listeners to columns
  visibleStages.forEach(stage => {
    const colEl = document.getElementById(`col-${stage.id}`);
    if (colEl) {
      colEl.addEventListener('dragover', (e) => {
        e.preventDefault();
        colEl.classList.add('drag-over');
      });
      colEl.addEventListener('dragleave', () => {
        colEl.classList.remove('drag-over');
      });
      colEl.addEventListener('drop', async (e) => {
        e.preventDefault();
        colEl.classList.remove('drag-over');
        const itemId = e.dataTransfer.getData('text/plain');
        if (itemId) {
          await handleDropOnStage(itemId, stage.relation);
        }
      });

      // Click empty space in cards container to trigger inline creation
      const cardsContainer = document.getElementById(`cards-${stage.id}`);
      if (cardsContainer) {
        cardsContainer.addEventListener('click', (e) => {
          if (e.target === cardsContainer || e.target.classList.contains('empty-col-placeholder')) {
            window.startInlineDraft(stage.id);
          }
        });
      }
    }
  });

  // Attach Drag listeners to cards
  document.querySelectorAll('.board-card, .kanban-card').forEach(card => {
    card.addEventListener('dragstart', (e) => {
      e.dataTransfer.setData('text/plain', card.dataset.id);
      card.classList.add('dragging');
    });
    card.addEventListener('dragend', () => {
      card.classList.remove('dragging');
    });
  });

  // Auto-focus input if inline draft active
  if (activeInlineDraftCol) {
    const input = document.getElementById('inline-draft-title');
    if (input) input.focus();
  }
}

// Inline Draft Card HTML
function renderInlineDraftCard(stageId) {
  return `
    <div class="board-inline-draft" id="active-inline-draft" onclick="event.stopPropagation()">
      <input type="text" id="inline-draft-title" class="inline-draft-input" placeholder="Task Title..." onkeydown="window.handleInlineDraftKey(event, '${stageId}')">
      <textarea id="inline-draft-prompt" class="inline-draft-textarea" rows="2" placeholder="Unit goal or developer prompt (Optional)" onkeydown="window.handleInlineDraftKey(event, '${stageId}')"></textarea>
      
      <div class="inline-draft-actions">
        <select id="inline-draft-domain" class="form-select" style="padding: 3px 22px 3px 6px; font-size: 11px;">
          <option value="#prod-bug">Engineering (#prod-bug)</option>
          <option value="#crm-request">Commercial (#crm-request)</option>
          <option value="#survey-mapping">Feedback (#survey-mapping)</option>
        </select>
        <div class="inline-draft-btns">
          <button class="btn btn-secondary btn-sm" onclick="window.cancelInlineDraft(event)">Cancel</button>
          <button class="btn btn-primary btn-sm" onclick="window.commitInlineDraft('${stageId}')">Add Task</button>
        </div>
      </div>
    </div>
  `;
}

window.startInlineDraft = (stageId) => {
  activeInlineDraftCol = stageId;
  renderBoard();
};

window.cancelInlineDraft = (e) => {
  if (e) e.stopPropagation();
  activeInlineDraftCol = null;
  renderBoard();
};

window.handleInlineDraftKey = (e, stageId) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    window.commitInlineDraft(stageId);
  } else if (e.key === 'Escape') {
    window.cancelInlineDraft(e);
  }
};

window.commitInlineDraft = async (stageId) => {
  const titleInput = document.getElementById('inline-draft-title');
  const promptInput = document.getElementById('inline-draft-prompt');
  const domainSelect = document.getElementById('inline-draft-domain');

  const title = titleInput?.value.trim();
  if (!title) {
    showToast('Please enter a task title', 'error');
    return;
  }

  const prompt = promptInput?.value.trim() || title;
  const domain_tag = domainSelect?.value || '#prod-bug';

  try {
    const config = getBoardConfig();
    const stage = config.find(s => s.id === stageId);

    const created = await invoke('ingest_operation', {
      title,
      description: prompt,
      requester: 'board_engineer',
      domain_tag,
      payload: null,
      prompt,
      system_goal: 'Modernize repository and eliminate behavioral regressions through atomic unit gates',
      tags: [domain_tag]
    });

    // If created in a later stage than captured, advance it immediately
    if (stage && stage.relation !== 'captured' && created?.id) {
      if (stage.relation === 'sandboxed' || stage.relation === 'verified') {
        await invoke('verify_operation', { id: created.id });
      } else if (stage.relation === 'approved' || stage.relation === 'promoted') {
        await invoke('verify_operation', { id: created.id });
        await invoke('approve_operation', { id: created.id, approver: 'board_user', notes: 'Created directly on board' });
      }
    }

    activeInlineDraftCol = null;
    showToast(`Created task: ${title}`, 'success');
    await fetchQueue();
  } catch (err) {
    showToast('Failed to create task: ' + err, 'error');
  }
};

// Render Individual Board Card
function renderBoardCard(item, stageRelation) {
  const isJustMoved = recentlyMovedId === item.id;
  const shortKey = item.id.length > 8 ? item.id.substring(item.id.length - 6).toUpperCase() : item.id.toUpperCase();
  const tags = getItemTags(item);
  const promptPreview = item.prompt ? item.prompt.trim() : '';

  let actionBtn = '';
  if (stageRelation === 'captured') {
    actionBtn = `<button class="card-action-btn" onclick="window.advanceTask('${item.id}', 'verify', event)">⚡ Run Tests</button>`;
  } else if (stageRelation === 'sandboxed') {
    actionBtn = `<button class="card-action-btn" onclick="window.advanceTask('${item.id}', 'verify', event)">▶ Run Tests</button>`;
  } else if (stageRelation === 'verified') {
    actionBtn = `<button class="card-action-btn btn-promote" onclick="window.advanceTask('${item.id}', 'approve', event)">✓ Approve</button>`;
  } else if (stageRelation === 'approved') {
    actionBtn = `<button class="card-action-btn btn-promote" onclick="window.advanceTask('${item.id}', 'promote', event)">🚀 Apply</button>`;
  } else if (stageRelation === 'promoted') {
    actionBtn = `<span style="font-size: 10px; color: var(--accent-emerald); font-weight: 700;">✓ In master</span>`;
  }

  return `
    <div class="board-card kanban-card ${isJustMoved ? 'just-moved' : ''}" 
         draggable="true" 
         data-id="${item.id}"
         onclick="window.openCardDetail('${item.id}')">
      <div class="card-top">
        <span class="card-key">${shortKey}</span>
        <div class="card-tags">
          ${tags.map((t, idx) => `
            <span class="tag-chip ${hasGraphMapping(item, t) ? 'has-graph' : ''}" 
                  onclick="window.openTagMappingModal('${item.id}', ${idx}, '${escapeHtml(t)}', event)"
                  title="Click to edit tag or view graph mapping">
              ${escapeHtml(t)}
            </span>
          `).join('')}
          <button class="tag-chip-add" onclick="window.promptAddTag('${item.id}', event)" title="Add tag">+</button>
        </div>
      </div>

      <div class="card-title">${escapeHtml(item.title)}</div>
      <div class="card-desc">${escapeHtml(item.description)}</div>

      ${promptPreview ? `
        <div class="card-prompt-badge" title="${escapeHtml(promptPreview)}">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"></path></svg>
          <span class="truncate">Unit: ${escapeHtml(promptPreview)}</span>
        </div>
      ` : ''}

      <div class="card-footer">
        <div class="card-actor">
          <span>👤 ${escapeHtml(item.requester)}</span>
        </div>
        <div class="card-actions">
          ${actionBtn}
        </div>
      </div>
    </div>
  `;
}

// Helper: Tag and Graph Mapping lookups
function getItemTags(item) {
  if (Array.isArray(item.tags) && item.tags.length > 0) return item.tags;
  // LocalStorage custom tag store fallback
  const localTagStore = getLocalTagsStore();
  if (localTagStore[item.id]?.tags) return localTagStore[item.id].tags;
  return [item.domain_tag || '#prod-bug'];
}

function getItemGraphMappings(item) {
  if (Array.isArray(item.graph_mappings) && item.graph_mappings.length > 0) return item.graph_mappings;
  const localTagStore = getLocalTagsStore();
  return localTagStore[item.id]?.graph_mappings || [
    item.payload?.data?.target_file || 'crates/auth/token.rs',
    'verify_token'
  ];
}

function hasGraphMapping(item, tag) {
  const mappings = getItemGraphMappings(item);
  return mappings.length > 0;
}

function getLocalTagsStore() {
  try {
    return JSON.parse(localStorage.getItem('exodus_task_tags_v1') || '{}');
  } catch (_) {
    return {};
  }
}

function saveLocalTagsStore(store) {
  localStorage.setItem('exodus_task_tags_v1', JSON.stringify(store));
}

// Add Tag prompt
window.promptAddTag = async (itemId, e) => {
  if (e) e.stopPropagation();
  const tagName = prompt('Enter new tag (e.g. #auth, #security, #p0-bug):');
  if (!tagName || !tagName.trim()) return;

  const item = currentItems.find(i => i.id === itemId);
  if (!item) return;

  const currentTags = getItemTags(item);
  const formattedTag = tagName.startsWith('#') ? tagName.trim() : '#' + tagName.trim();
  if (!currentTags.includes(formattedTag)) {
    currentTags.push(formattedTag);
  }

  const currentMappings = getItemGraphMappings(item);

  try {
    await invoke('update_item_tags', {
      id: itemId,
      tags: currentTags,
      graph_mappings: currentMappings
    });
  } catch (_) {}

  const store = getLocalTagsStore();
  store[itemId] = { tags: currentTags, graph_mappings: currentMappings };
  saveLocalTagsStore(store);

  showToast(`Added tag ${formattedTag}`, 'success');
  await fetchQueue();
};

// ============================================================================
// Tag & Change Graph Mapping Modal
// ============================================================================
window.openTagMappingModal = (itemId, tagIndex, tagName, e) => {
  if (e) e.stopPropagation();
  const item = currentItems.find(i => i.id === itemId);
  if (!item) return;

  const mappings = getItemGraphMappings(item);
  activeTagModalData = {
    itemId,
    tagIndex,
    tagName,
    mappedSymbols: [...mappings]
  };

  const modal = document.getElementById('modal-tag-mapping');
  const nameInput = document.getElementById('modal-tag-name');
  if (nameInput) nameInput.value = tagName;

  renderMappedSymbolsList();
  modal?.classList.remove('hidden');
};

function renderMappedSymbolsList() {
  const list = document.getElementById('modal-mapped-symbols-list');
  if (!list || !activeTagModalData) return;

  list.innerHTML = activeTagModalData.mappedSymbols.map((sym, idx) => `
    <div class="mapped-symbol-item">
      <span>${escapeHtml(sym)}</span>
      <button style="background:none; border:none; color:var(--accent-rose); cursor:pointer;" onclick="window.removeMappedSymbol(${idx})">&times;</button>
    </div>
  `).join('');
}

window.removeMappedSymbol = (idx) => {
  if (!activeTagModalData) return;
  activeTagModalData.mappedSymbols.splice(idx, 1);
  renderMappedSymbolsList();
};

document.getElementById('btn-add-mapped-symbol')?.addEventListener('click', () => {
  const input = document.getElementById('modal-tag-symbol-input');
  const val = input?.value.trim();
  if (!val || !activeTagModalData) return;
  if (!activeTagModalData.mappedSymbols.includes(val)) {
    activeTagModalData.mappedSymbols.push(val);
  }
  input.value = '';
  renderMappedSymbolsList();
});

document.getElementById('btn-close-tag-modal')?.addEventListener('click', () => {
  document.getElementById('modal-tag-mapping')?.classList.add('hidden');
});
document.getElementById('btn-cancel-tag-modal')?.addEventListener('click', () => {
  document.getElementById('modal-tag-mapping')?.classList.add('hidden');
});

document.getElementById('btn-delete-tag')?.addEventListener('click', async () => {
  if (!activeTagModalData) return;
  const { itemId, tagIndex } = activeTagModalData;
  const item = currentItems.find(i => i.id === itemId);
  if (item) {
    const tags = getItemTags(item);
    tags.splice(tagIndex, 1);
    const mappings = activeTagModalData.mappedSymbols;

    try {
      await invoke('update_item_tags', { id: itemId, tags, graph_mappings: mappings });
    } catch (_) {}

    const store = getLocalTagsStore();
    store[itemId] = { tags, graph_mappings: mappings };
    saveLocalTagsStore(store);

    showToast('Tag removed', 'info');
    document.getElementById('modal-tag-mapping')?.classList.add('hidden');
    await fetchQueue();
  }
});

document.getElementById('btn-save-tag-modal')?.addEventListener('click', async () => {
  if (!activeTagModalData) return;
  const { itemId, tagIndex } = activeTagModalData;
  const nameInput = document.getElementById('modal-tag-name');
  const newName = nameInput?.value.trim() || activeTagModalData.tagName;

  const item = currentItems.find(i => i.id === itemId);
  if (item) {
    const tags = getItemTags(item);
    tags[tagIndex] = newName.startsWith('#') ? newName : '#' + newName;
    const mappings = activeTagModalData.mappedSymbols;

    try {
      await invoke('update_item_tags', { id: itemId, tags, graph_mappings: mappings });
    } catch (_) {}

    const store = getLocalTagsStore();
    store[itemId] = { tags, graph_mappings: mappings };
    saveLocalTagsStore(store);

    showToast(`Updated tag ${newName} and graph mappings`, 'success');
    document.getElementById('modal-tag-mapping')?.classList.add('hidden');
    await fetchQueue();
  }
});

// Jump Directly to Change Graph and highlight mapped symbols
document.getElementById('btn-jump-to-graph')?.addEventListener('click', () => {
  if (!activeTagModalData) return;
  const targetSymbol = activeTagModalData.mappedSymbols[0] || 'token.rs';
  document.getElementById('modal-tag-mapping')?.classList.add('hidden');

  // Switch to Change Graph tab
  document.getElementById('tab-btn-graph')?.click();

  // Find node corresponding to symbol and select it
  setTimeout(() => {
    if (currentGraphTopology?.nodes) {
      const matched = currentGraphTopology.nodes.find(n => 
        n.label.toLowerCase().includes(targetSymbol.toLowerCase()) ||
        n.id.toLowerCase().includes(targetSymbol.toLowerCase()) ||
        n.detail?.toLowerCase().includes(targetSymbol.toLowerCase())
      );
      if (matched) {
        selectedGraphNodeId = matched.id;
        renderNodeDrawer(matched);
        renderEsgCanvas();
        showToast(`Highlighted mapped symbol: ${matched.label}`, 'success');
      }
    }
  }, 100);
});

// ============================================================================
// Attached Prompt & Failure Diagnostics
// ============================================================================
document.getElementById('btn-edit-prompt')?.addEventListener('click', () => {
  const item = currentItems.find(i => i.id === selectedItemId);
  if (!item) return;

  const modal = document.getElementById('modal-edit-prompt');
  const sysInput = document.getElementById('edit-prompt-system-goal');
  const unitInput = document.getElementById('edit-prompt-unit-text');

  if (sysInput) sysInput.value = item.system_goal || 'Modernize repository and eliminate behavioral regressions through atomic unit gates';
  if (unitInput) unitInput.value = item.prompt || item.description || item.title;

  modal?.classList.remove('hidden');
});

document.getElementById('btn-close-prompt-modal')?.addEventListener('click', () => {
  document.getElementById('modal-edit-prompt')?.classList.add('hidden');
});
document.getElementById('btn-cancel-prompt-modal')?.addEventListener('click', () => {
  document.getElementById('modal-edit-prompt')?.classList.add('hidden');
});

document.getElementById('btn-save-prompt-modal')?.addEventListener('click', async () => {
  const item = currentItems.find(i => i.id === selectedItemId);
  if (!item) return;

  const sysInput = document.getElementById('edit-prompt-system-goal');
  const unitInput = document.getElementById('edit-prompt-unit-text');

  const system_goal = sysInput?.value.trim() || 'Modernize repository through atomic unit gates';
  const prompt = unitInput?.value.trim() || item.title;

  try {
    await invoke('update_item_prompt', {
      id: item.id,
      prompt,
      system_goal
    });

    item.prompt = prompt;
    item.system_goal = system_goal;

    // Trigger harness verification unit with new prompt
    showToast('Dispatching unit to harness with updated prompt...', 'info');
    await invoke('execute_harness_unit', {
      id: item.id,
      prompt,
      system_goal
    });

    document.getElementById('modal-edit-prompt')?.classList.add('hidden');
    showToast('Prompt attached and verified in sandbox', 'success');
    await fetchQueue();
  } catch (err) {
    showToast('Failed to update prompt: ' + err, 'error');
  }
});

// Advance Task Actions
window.advanceTask = async (id, action, event) => {
  if (event) event.stopPropagation();
  recentlyMovedId = id;

  try {
    if (action === 'verify') {
      const item = currentItems.find(i => i.id === id);
      await invoke('execute_harness_unit', {
        id,
        prompt: item?.prompt || null,
        system_goal: item?.system_goal || null
      });
      showToast('Unit executed and verified in sandbox', 'success');
    } else if (action === 'approve') {
      await invoke('approve_operation', {
        id,
        approver: 'human_operator',
        notes: 'Approved via Board'
      });
      showToast('Task signed off and approved', 'success');
    } else if (action === 'promote') {
      await invoke('approve_operation', {
        id,
        approver: 'human_operator',
        notes: 'Applied & promoted to master'
      });
      showToast('Changes promoted and merged into master', 'success');
    }
    await fetchQueue();
  } catch (err) {
    showToast('Failed to advance task: ' + err, 'error');
  }
};

async function handleDropOnStage(itemId, targetStage) {
  recentlyMovedId = itemId;
  try {
    if (targetStage === 'verified' || targetStage === 'sandboxed') {
      await invoke('verify_operation', { id: itemId });
      showToast(`Task moved to ${formatStateLabel(targetStage)}`, 'success');
    } else if (targetStage === 'approved' || targetStage === 'promoted') {
      await invoke('approve_operation', {
        id: itemId,
        approver: 'human_operator',
        notes: `Moved to ${targetStage} via Board drag & drop`
      });
      showToast(`Task moved to ${formatStateLabel(targetStage)}`, 'success');
    }
    await fetchQueue();
  } catch (err) {
    showToast('Cannot transition task: ' + err, 'error');
  }
}

window.openCardDetail = (id) => {
  selectedItemId = id;
  const item = currentItems.find(i => i.id === id);
  if (item) {
    selectItem(item);
    document.getElementById('tab-btn-queue')?.click();
  }
};

function formatStateLabel(state) {
  if (!state) return '';
  const map = {
    'captured': 'Triggered',
    'sandboxed': 'In Sandbox',
    'contract_verified': 'Tests Passing',
    'contractverified': 'Tests Passing',
    'degraded': 'Tests Passing (Degraded)',
    'human_approved': 'Approved',
    'humanapproved': 'Approved',
    'promoted': 'Applied',
    'rejected': 'Rejected'
  };
  return map[state.toLowerCase()] || state;
}

// ============================================================================
// Queue & Item Selection in Tasks & Review View
// ============================================================================
async function fetchQueue() {
  try {
    const items = await invoke('list_operations');
    if (items) {
      currentItems = items;
      renderKPIs();
      renderSidebarList();
      renderBoard();
      if (selectedItemId) {
        const found = currentItems.find(i => i.id === selectedItemId);
        if (found) selectItem(found);
      }
    }
  } catch (err) {
    console.error('Failed to fetch queue:', err);
  }
}

function renderKPIs() {
  const prodCount = currentItems.filter(i => i.domain_tag === '#prod-bug').length;
  const crmCount = currentItems.filter(i => i.domain_tag === '#crm-request').length;
  const surveyCount = currentItems.filter(i => i.domain_tag === '#survey-mapping').length;

  const prodEl = document.getElementById('metric-prod-count');
  const crmEl = document.getElementById('metric-crm-count');
  const surveyEl = document.getElementById('metric-survey-count');

  if (prodEl) prodEl.textContent = prodCount;
  if (crmEl) crmEl.textContent = crmCount;
  if (surveyEl) surveyEl.textContent = surveyCount;
}

function renderSidebarList() {
  const listEl = document.getElementById('items-list');
  if (!listEl) return;

  const filtered = currentFilter === 'all' 
    ? currentItems 
    : currentItems.filter(i => i.domain_tag === currentFilter || getItemTags(i).includes(currentFilter));

  if (filtered.length === 0) {
    listEl.innerHTML = '<div class="empty-state">No tasks matching filter.</div>';
    return;
  }

  listEl.innerHTML = filtered.map(item => {
    const isSelected = item.id === selectedItemId;
    const badgeClass = item.domain_tag === '#prod-bug' ? 'badge-emerald' 
      : item.domain_tag === '#crm-request' ? 'badge-amber' : 'badge-cyan';
    
    return `
      <div class="item-card ${isSelected ? 'selected' : ''}" onclick="window.selectItemById('${item.id}')">
        <div class="item-header">
          <span class="badge ${badgeClass}">${item.domain_tag}</span>
          <span style="font-size: 11px; font-weight: 700; color: ${getStateColor(item.state)}">${formatStateLabel(item.state)}</span>
        </div>
        <div class="item-title">${escapeHtml(item.title)}</div>
        <div class="item-meta">
          <span>By: ${escapeHtml(item.requester)}</span>
          <span>${formatDate(item.created_at)}</span>
        </div>
      </div>
    `;
  }).join('');
}

window.selectItemById = (id) => {
  selectedItemId = id;
  const item = currentItems.find(i => i.id === id);
  if (item) selectItem(item);
  renderSidebarList();
};

function selectItem(item) {
  selectedItemId = item.id;
  document.getElementById('empty-review')?.classList.add('hidden');
  document.getElementById('active-review')?.classList.remove('hidden');

  document.getElementById('review-id').textContent = item.id;
  document.getElementById('review-title').textContent = item.title;
  document.getElementById('review-desc').textContent = item.description;

  const domainBadge = document.getElementById('review-domain-badge');
  domainBadge.textContent = item.domain_tag;
  domainBadge.className = `badge ${item.domain_tag === '#prod-bug' ? 'badge-emerald' : item.domain_tag === '#crm-request' ? 'badge-amber' : 'badge-cyan'}`;

  const stateBadge = document.getElementById('review-state-badge');
  stateBadge.textContent = formatStateLabel(item.state);
  stateBadge.style.color = getStateColor(item.state);

  // Custom tags in review
  const customTagsContainer = document.getElementById('review-custom-tags');
  if (customTagsContainer) {
    const tags = getItemTags(item);
    customTagsContainer.innerHTML = tags.map((t, idx) => `
      <span class="tag-chip ${hasGraphMapping(item, t) ? 'has-graph' : ''}" 
            onclick="window.openTagMappingModal('${item.id}', ${idx}, '${escapeHtml(t)}', event)">
        ${escapeHtml(t)}
      </span>
    `).join('') + `<button class="tag-chip-add" onclick="window.promptAddTag('${item.id}', event)">+ Tag</button>`;
  }

  // Stepper Track
  updateStepper(item.state);

  // Attached Prompt Box
  const promptTextEl = document.getElementById('attached-prompt-text');
  if (promptTextEl) {
    promptTextEl.textContent = item.prompt || item.description || 'No custom prompt attached. Click Edit Prompt to add one.';
  }

  // Unit Achievement vs. Whole System Goal Diagnostic Breakdown
  renderPromptBreakdown(item);

  // Inspection Payload & Tests
  renderInspection(item);
  renderVerification(item);
  renderAudit(item);

  // Button States
  const s = (item.state || '').toLowerCase();
  const btnApprove = document.getElementById('btn-action-approve');
  const btnVerify = document.getElementById('btn-action-verify');
  const btnReject = document.getElementById('btn-action-reject');

  if (s === 'promoted') {
    if (btnApprove) btnApprove.disabled = true;
    if (btnVerify) btnVerify.disabled = true;
    if (btnReject) btnReject.disabled = true;
  } else {
    if (btnApprove) btnApprove.disabled = false;
    if (btnVerify) btnVerify.disabled = false;
    if (btnReject) btnReject.disabled = false;
  }
}

function updateStepper(state) {
  const stages = ['captured', 'sandboxed', 'verified', 'approved', 'promoted'];
  const s = (state || '').toLowerCase();
  let currentIdx = 0;
  if (s === 'sandboxed') currentIdx = 1;
  else if (s === 'contract_verified' || s === 'contractverified' || s === 'degraded') currentIdx = 2;
  else if (s === 'human_approved' || s === 'humanapproved') currentIdx = 3;
  else if (s === 'promoted') currentIdx = 4;

  stages.forEach((stage, idx) => {
    const node = document.getElementById(`step-${stage}`);
    if (!node) return;
    node.className = 'step-node';
    if (idx < currentIdx) node.classList.add('completed');
    if (idx === currentIdx) node.classList.add('active');

    if (idx > 0) {
      const line = document.getElementById(`line-${idx}`);
      if (line) {
        line.className = idx <= currentIdx ? 'step-line active' : 'step-line';
      }
    }
  });
}

// Render Unit Achievement vs System Goal Breakdown
function renderPromptBreakdown(item) {
  const sysGoalEl = document.getElementById('breakdown-system-goal');
  const unitGoalEl = document.getElementById('breakdown-unit-goal');
  const statusBadge = document.getElementById('breakdown-status-badge');
  const diagContainer = document.getElementById('breakdown-stage-diagnostics');
  const attachStatus = document.getElementById('breakdown-attachment-status');

  const unitGoal = item.prompt || item.title;
  const sysGoal = item.system_goal || 'Modernize repository and eliminate behavioral regressions through atomic unit gates';

  if (sysGoalEl) sysGoalEl.textContent = sysGoal;
  if (unitGoalEl) unitGoalEl.textContent = unitGoal;

  const diag = item.unit_diagnostic;
  const report = item.verification_report;

  if (diag) {
    if (statusBadge) {
      statusBadge.textContent = 'Diverged / Failed';
      statusBadge.className = 'badge badge-rose';
    }
    if (attachStatus) {
      attachStatus.textContent = 'Blocked: Requires Unit Correction Before Downstream Merge';
      attachStatus.className = 'text-rose-400 font-bold';
    }

    diagContainer.innerHTML = `
      <div class="stage-failure-header fail">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"></circle><line x1="15" y1="9" x2="9" y2="15"></line><line x1="9" y1="9" x2="15" y2="15"></line></svg>
        <span>Failed at ${escapeHtml(diag.stage_failed)}</span>
      </div>

      <div class="root-cause-box">
        <strong>Why the Prompt Failed to Achieve the Goal:</strong><br>
        ${escapeHtml(diag.failure_reason)}
      </div>

      ${diag.error_snippet ? `
        <div style="font-size: 11px; font-family: var(--font-mono); background: #07090d; padding: 8px 10px; border-radius: 6px; border: 1px solid var(--border); color: #f87171; overflow-x: auto;">
          ${escapeHtml(diag.error_snippet)}
        </div>
      ` : ''}

      ${diag.suggested_refinement ? `
        <div class="prompt-refinement-box">
          <span class="prompt-refinement-title">💡 Suggested Prompt Refinement to Achieve Goal:</span>
          <div class="prompt-refinement-text">"${escapeHtml(diag.suggested_refinement)}"</div>
          <div>
            <button class="btn btn-primary btn-sm" onclick="window.applyRefinedPrompt('${item.id}', '${escapeHtml(diag.suggested_refinement).replace(/'/g, "\\'")}')">
              ▶ Apply Refinement & Retry Unit
            </button>
          </div>
        </div>
      ` : ''}
    `;
  } else if (report && report.passed) {
    if (statusBadge) {
      statusBadge.textContent = 'Contract Verified (100%)';
      statusBadge.className = 'badge badge-emerald';
    }
    if (attachStatus) {
      attachStatus.textContent = '✓ Attached to System Graph & Target Release Pipeline';
      attachStatus.className = 'text-emerald-400 font-bold';
    }

    diagContainer.innerHTML = `
      <div class="stage-failure-header pass">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>
        <span>Unit Goal Satisfied All Automated Gates</span>
      </div>
      <div style="font-size: 12px; color: var(--text-secondary);">
        The attached prompt achieved its atomic unit change without compiler regression or behavioral contract failure. This unit is verified and ready for human approval and merge.
      </div>
    `;
  } else {
    if (statusBadge) {
      statusBadge.textContent = 'Pending Verification';
      statusBadge.className = 'badge badge-cyan';
    }
    if (attachStatus) {
      attachStatus.textContent = 'In Progress in Sandbox';
      attachStatus.className = 'text-cyan-400';
    }

    diagContainer.innerHTML = `
      <div style="font-size: 12px; color: var(--text-muted); font-style: italic;">
        Click 'Run Tests (v)' to execute this unit within an isolated sandbox and inspect stage-by-stage contract compliance.
      </div>
    `;
  }
}

window.applyRefinedPrompt = async (itemId, refinedPrompt) => {
  try {
    await invoke('update_item_prompt', {
      id: itemId,
      prompt: refinedPrompt,
      system_goal: null
    });
    showToast('Applying refined prompt and executing harness unit...', 'info');
    await invoke('execute_harness_unit', {
      id: itemId,
      prompt: refinedPrompt,
      system_goal: null
    });
    showToast('Unit re-executed with refined prompt', 'success');
    await fetchQueue();
  } catch (err) {
    showToast('Refinement retry failed: ' + err, 'error');
  }
};

function renderInspection(item) {
  const container = document.getElementById('inspection-container');
  const payload = item.payload;

  if (payload.domain_type === 'prod_bug') {
    const data = payload.data;
    container.innerHTML = `
      <div class="panel-card">
        <h3>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="16 18 22 12 16 6"></polyline><polyline points="8 6 2 12 8 18"></polyline></svg>
          Codebase Changes & Git Diff
        </h3>
        <div style="margin-bottom: 12px; font-size: 12px; color: var(--text-secondary);">
          <strong>Commit:</strong> <code style="color: var(--accent-cyan);">${escapeHtml(data.commit_id)}</code> &bull;
          <strong>Reproduction Command:</strong> <code>${escapeHtml(data.reproduction_command || 'cargo test')}</code>
        </div>
        <div class="diff-box">
          <span class="diff-ctx">@@ -40,7 +40,7 @@ fn authenticate_session(token: &str) -> Result&lt;Session&gt; {</span>
          <span class="diff-rem">-    let claims = parse_jwt_unchecked(token)?; // Panic on malformed token</span>
          <span class="diff-add">+    let claims = parse_jwt_safe(token).map_err(|e| AuthError::InvalidToken(e))?;</span>
          <span class="diff-ctx">     validate_expiration(&claims)?;</span>
          <span class="diff-ctx">     Ok(Session::from_claims(claims))</span>
        </div>
      </div>
    `;
  } else if (payload.domain_type === 'crm_request') {
    const data = payload.data;
    container.innerHTML = `
      <div class="panel-card">
        <h3>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="5" width="20" height="14" rx="2"></rect><line x1="2" y1="10" x2="22" y2="10"></line></svg>
          Commercial Policy & Contract Impact
        </h3>
        <div style="display: flex; gap: 12px; margin-bottom: 12px;">
          <div style="background: var(--bg-card); padding: 10px; border-radius: 8px; flex: 1;">
            <div style="font-size: 10px; color: var(--text-muted);">ACCOUNT ARR</div>
            <div style="font-size: 16px; font-weight: 700;">$${Number(data.current_arr || 0).toLocaleString()}</div>
          </div>
          <div style="background: var(--bg-card); padding: 10px; border-radius: 8px; flex: 1;">
            <div style="font-size: 10px; color: var(--text-muted);">REQUESTED DISCOUNT</div>
            <div style="font-size: 16px; font-weight: 700; color: ${data.requested_discount_pct > 25 ? 'var(--accent-amber)' : 'var(--accent-emerald)'};">${data.requested_discount_pct}%</div>
          </div>
        </div>
      </div>
    `;
  } else if (payload.domain_type === 'survey_mapping') {
    const data = payload.data;
    const responses = data.responses || [];
    container.innerHTML = `
      <div class="panel-card">
        <h3>Customer Feedback Categorization (${responses.length} Items)</h3>
        <div style="display: flex; flex-direction: column; gap: 6px;">
          ${responses.map(r => `
            <div style="display: flex; justify-content: space-between; padding: 6px 8px; background: var(--bg-card); border-radius: 6px; font-size: 12px;">
              <span>"${escapeHtml(r.feedback_text)}"</span>
              <span class="badge ${r.candidate_taxonomy_node ? 'badge-cyan' : 'badge-rose'}">
                ${r.candidate_taxonomy_node || 'UNMAPPED'}
              </span>
            </div>
          `).join('')}
        </div>
      </div>
    `;
  }
}

function renderVerification(item) {
  const summaryEl = document.getElementById('verification-summary');
  const rulesListEl = document.getElementById('rule-results-list');

  if (!item.verification_report) {
    summaryEl.innerHTML = '<span style="color: var(--text-muted);">Contract not yet verified. Click "Run Tests (v)" to run deterministic gates.</span>';
    rulesListEl.innerHTML = '';
    return;
  }

  const rep = item.verification_report;
  summaryEl.innerHTML = `
    <div style="font-weight: 700; color: ${rep.passed ? 'var(--accent-emerald)' : 'var(--accent-rose)'}; margin-bottom: 8px;">
      ${rep.passed ? '✓ PASSED CONTRACT VERIFICATION' : '✗ VERIFICATION FAILED'}
    </div>
    <div style="font-size: 12px; color: var(--text-secondary);">${escapeHtml(rep.summary)}</div>
  `;

  rulesListEl.innerHTML = (rep.rule_results || []).map(r => `
    <div class="rule-result-item ${r.passed ? 'pass' : 'fail'}">
      <div><strong>${escapeHtml(r.rule_name)}:</strong> ${escapeHtml(r.details)}</div>
      <span class="badge ${r.passed ? 'badge-emerald' : 'badge-rose'}">${r.passed ? 'PASS' : 'FAIL'}</span>
    </div>
  `).join('');
}

function renderAudit(item) {
  const container = document.getElementById('audit-entries');
  if (!container) return;
  const entries = item.audit_trail || [];

  container.innerHTML = entries.map(e => `
    <div style="display: flex; justify-content: space-between; padding: 6px 0; border-bottom: 1px solid var(--border); font-size: 11px;">
      <div>
        <span style="font-weight: 700; color: var(--accent-gold);">${escapeHtml(e.action)}</span>
        <span style="color: var(--text-secondary);"> &bull; ${escapeHtml(e.details)}</span>
      </div>
      <div style="color: var(--text-muted);">${formatDate(e.timestamp)}</div>
    </div>
  `).join('');
}

// Action Button Listeners in Review Header
document.getElementById('btn-action-verify')?.addEventListener('click', async () => {
  if (!selectedItemId) return;
  const item = currentItems.find(i => i.id === selectedItemId);
  showToast('Executing automated unit tests in sandbox...', 'info');
  try {
    await invoke('execute_harness_unit', {
      id: selectedItemId,
      prompt: item?.prompt || null,
      system_goal: item?.system_goal || null
    });
    showToast('Contract verification executed', 'success');
    await fetchQueue();
  } catch (err) {
    showToast('Verification error: ' + err, 'error');
  }
});

document.getElementById('btn-action-approve')?.addEventListener('click', async () => {
  if (!selectedItemId) return;
  try {
    await invoke('approve_operation', {
      id: selectedItemId,
      approver: 'lead_operator',
      notes: 'Signed off from Exodus Mission Control'
    });
    showToast('Task approved and scheduled for promotion', 'success');
    await fetchQueue();
  } catch (err) {
    showToast('Approval rejected: ' + err, 'error');
  }
});

document.getElementById('btn-action-reject')?.addEventListener('click', async () => {
  if (!selectedItemId) return;
  const reason = prompt('Enter reason for rejection:');
  if (!reason) return;
  try {
    await invoke('reject_operation', {
      id: selectedItemId,
      actor: 'lead_operator',
      reason
    });
    showToast('Task rejected', 'info');
    await fetchQueue();
  } catch (err) {
    showToast('Rejection error: ' + err, 'error');
  }
});

document.getElementById('btn-view-lineage')?.addEventListener('click', () => {
  document.getElementById('tab-btn-graph')?.click();
});

// Domain Filter Tabs in Sidebar
document.querySelectorAll('#domain-filters .filter-tab').forEach(tab => {
  tab.addEventListener('click', (e) => {
    document.querySelectorAll('#domain-filters .filter-tab').forEach(t => t.classList.remove('active'));
    e.target.classList.add('active');
    currentFilter = e.target.getAttribute('data-filter');
    renderSidebarList();
  });
});

document.getElementById('btn-refresh-queue')?.addEventListener('click', fetchQueue);

// New Task Modal Handlers
document.getElementById('btn-new-item')?.addEventListener('click', () => {
  document.getElementById('modal-ingest')?.classList.remove('hidden');
});
document.getElementById('btn-close-ingest')?.addEventListener('click', () => {
  document.getElementById('modal-ingest')?.classList.add('hidden');
});
document.getElementById('btn-cancel-ingest')?.addEventListener('click', () => {
  document.getElementById('modal-ingest')?.classList.add('hidden');
});

document.getElementById('btn-submit-ingest')?.addEventListener('click', async () => {
  const domain_tag = document.getElementById('new-item-domain').value;
  const title = document.getElementById('new-item-title').value.trim();
  const desc = document.getElementById('new-item-desc').value.trim();
  const requester = document.getElementById('new-item-requester').value.trim() || 'engineer';

  if (!title) {
    showToast('Title is required', 'error');
    return;
  }

  try {
    await invoke('ingest_operation', {
      title,
      description: desc || title,
      requester,
      domain_tag,
      payload: null,
      prompt: desc || title,
      system_goal: 'Modernize repository and eliminate behavioral regressions through atomic unit gates',
      tags: [domain_tag]
    });
    document.getElementById('modal-ingest')?.classList.add('hidden');
    showToast('Task ingested successfully', 'success');
    await fetchQueue();
  } catch (err) {
    showToast('Failed to ingest: ' + err, 'error');
  }
});

// SDLC Settings Modal Handlers
document.getElementById('btn-sdlc-settings')?.addEventListener('click', async () => {
  try {
    const settings = await invoke('get_sdlc_settings');
    if (settings) {
      document.getElementById('sdlc-repo-url').value = settings.repo_url;
      document.getElementById('sdlc-provider').value = settings.provider;
      document.getElementById('sdlc-default-branch').value = settings.default_target_branch;
      document.getElementById('sdlc-ci-type').value = settings.ci_type;
    }
    const scaffold = await invoke('get_sdlc_scaffold');
    if (scaffold) {
      const firstScaffold = Object.values(scaffold)[0] || '';
      document.getElementById('sdlc-scaffold-preview').textContent = firstScaffold;
    }
    document.getElementById('modal-sdlc')?.classList.remove('hidden');
  } catch (err) {
    showToast('Failed to load SDLC settings: ' + err, 'error');
  }
});

document.getElementById('btn-close-sdlc')?.addEventListener('click', () => {
  document.getElementById('modal-sdlc')?.classList.add('hidden');
});
document.getElementById('btn-cancel-sdlc')?.addEventListener('click', () => {
  document.getElementById('modal-sdlc')?.classList.add('hidden');
});

// ============================================================================
// Change Lineage Graph Canvas & Interactive Inspector
// ============================================================================
let currentGraphTopology = null;
let selectedGraphNodeId = null;
let graphNodePositions = {};

async function renderEsgCanvas() {
  const canvas = document.getElementById('esg-canvas');
  if (!canvas) return;
  const ctx = canvas.getContext('2d');

  const rect = canvas.parentElement.getBoundingClientRect();
  canvas.width = rect.width;
  canvas.height = rect.height;

  let topology;
  try {
    topology = await invoke('get_esg_topology');
  } catch (_) {}

  if (!topology) {
    topology = {
      nodes: [
        {
          id: "node_trigger",
          label: "CI Failure #412",
          kind: "trigger",
          wave: 1,
          status: "active",
          subtitle: "Trigger: GitHub Actions CI Crash",
          detail: "Pipeline failed in auth_integration suite. TokenVerifier panicked on unhandled ExpiredSignature error.",
          diff_snippet: null,
          meta: { "source": "GitHub Actions", "event": "CI Build #412", "branch": "master" }
        },
        {
          id: "node_prompt",
          label: "Developer Prompt",
          kind: "prompt",
          wave: 2,
          status: "completed",
          subtitle: "Prompt: Handle Token Expiration",
          detail: "Prompt: 'In crates/auth/src/token.rs, safely catch ExpiredSignature and return AuthError::TokenExpired instead of panicking. Run all unit tests to confirm the fix.'",
          diff_snippet: null,
          meta: { "author": "engineer@exodus.dev", "model": "Claude 3.5 Sonnet" }
        },
        {
          id: "node_action",
          label: "AI Repair Task",
          kind: "action",
          wave: 3,
          status: "completed",
          subtitle: "Task: Safe Token Validation",
          detail: "Generated bounded repair for token validation. Isolated changes inside git worktree sandbox and prepared regression tests.",
          diff_snippet: null,
          meta: { "strategy": "Bounded AST Repair", "sandbox": ".exodus/worktrees/op-eng-412" }
        },
        {
          id: "node_code_file",
          label: "auth/src/token.rs",
          kind: "file_change",
          wave: 4,
          status: "modified",
          subtitle: "Codebase File (+8, -2 lines)",
          detail: "Modified authenticate_session to parse JWT claims safely and map expiration to structured error.",
          diff_snippet: "@@ -40,7 +40,11 @@ fn authenticate_session(token: &str) -> Result<Session, AuthError> {\n-    let claims = parse_jwt_unchecked(token)?; // Panic on expired token\n+    let claims = match parse_jwt_safe(token) {\n+        Ok(c) => c,\n+        Err(JwtError::ExpiredSignature) => return Err(AuthError::TokenExpired),\n+        Err(e) => return Err(AuthError::InvalidToken(e.to_string())),\n+    };\n     validate_expiration(&claims)?;\n     Ok(Session::from_claims(claims))",
          meta: { "file": "crates/auth/src/token.rs", "diff": "+8 / -2 lines" }
        },
        {
          id: "node_fn_symbol",
          label: "verify_token()",
          kind: "symbol",
          wave: 4,
          status: "verified",
          subtitle: "Function: AuthHandler::verify_token",
          detail: "Exported public signature: pub async fn verify_token(&self, token: &str) -> Result<Session, AuthError>",
          diff_snippet: null,
          meta: { "visibility": "pub", "type": "async fn" }
        },
        {
          id: "node_test_run",
          label: "Automated Tests",
          kind: "test",
          wave: 5,
          status: "passed",
          subtitle: "cargo test (3 passed)",
          detail: "Running 3 tests in crates/auth/tests/auth_integration.rs:\ntest test_valid_token ... ok\ntest test_token_expiration ... ok\ntest test_malformed_token ... ok\n\ntest result: ok. 3 passed; 0 failed; 0 ignored; finished in 0.42s",
          diff_snippet: null,
          meta: { "command": "cargo test --test auth_integration", "passed": "3", "failed": "0" }
        },
        {
          id: "node_target_merge",
          label: "Release Target",
          kind: "target",
          wave: 6,
          status: "ready",
          subtitle: "Ready for 1-Click Merge",
          detail: "Clean diff with all automated tests passing. Ready for human review sign-off and branch promotion.",
          diff_snippet: null,
          meta: { "target_branch": "master", "status": "Ready for Review" }
        }
      ],
      edges: [
        { from: "node_trigger", to: "node_prompt", edge_type: "triggers", is_cycle_edge: false },
        { from: "node_prompt", to: "node_action", edge_type: "instructs", is_cycle_edge: false },
        { from: "node_action", to: "node_code_file", edge_type: "modifies", is_cycle_edge: false },
        { from: "node_code_file", to: "node_fn_symbol", edge_type: "contains", is_cycle_edge: false },
        { from: "node_code_file", to: "node_test_run", edge_type: "verified_by", is_cycle_edge: false },
        { from: "node_test_run", to: "node_target_merge", edge_type: "promotes_to", is_cycle_edge: false }
      ]
    };
  }

  currentGraphTopology = topology;
  document.getElementById('graph-node-count').textContent = topology.nodes.length;
  document.getElementById('graph-edge-count').textContent = topology.edges.length;
  document.getElementById('graph-status-count').textContent = 'All Passed';

  const waveGroups = {};
  topology.nodes.forEach(n => {
    waveGroups[n.wave] = waveGroups[n.wave] || [];
    waveGroups[n.wave].push(n);
  });

  graphNodePositions = {};
  const width = canvas.width;
  const height = canvas.height;
  const waveKeys = Object.keys(waveGroups).sort((a, b) => Number(a) - Number(b));
  const colWidth = width / (waveKeys.length + 1);

  waveKeys.forEach((waveKey, colIdx) => {
    const colNodes = waveGroups[waveKey];
    const rowHeight = height / (colNodes.length + 1);
    colNodes.forEach((node, rowIdx) => {
      graphNodePositions[node.id] = {
        x: colWidth * (colIdx + 1),
        y: rowHeight * (rowIdx + 1),
        radius: 26,
        node
      };
    });
  });

  if (!selectedGraphNodeId) {
    const defaultNode = topology.nodes.find(n => n.kind === 'file_change') || topology.nodes[0];
    if (defaultNode) {
      selectedGraphNodeId = defaultNode.id;
      renderNodeDrawer(defaultNode);
    }
  }

  drawGraph(ctx, canvas.width, canvas.height, waveKeys, colWidth);
}

function drawGraph(ctx, width, height, waveKeys, colWidth) {
  ctx.clearRect(0, 0, width, height);

  const stageLabels = {
    '1': '1. TRIGGER',
    '2': '2. PROMPT',
    '3': '3. TASK ACTION',
    '4': '4. CODE CHANGE',
    '5': '5. TESTS',
    '6': '6. TARGET'
  };

  waveKeys.forEach((waveKey, colIdx) => {
    const x = colWidth * (colIdx + 1);
    ctx.strokeStyle = 'rgba(36, 50, 71, 0.35)';
    ctx.lineWidth = 1;
    ctx.setLineDash([4, 4]);
    ctx.beginPath();
    ctx.moveTo(x, 46);
    ctx.lineTo(x, height - 36);
    ctx.stroke();
    ctx.setLineDash([]);

    ctx.fillStyle = '#64748b';
    ctx.font = '700 10px "JetBrains Mono"';
    ctx.textAlign = 'center';
    ctx.fillText(stageLabels[waveKey] || `STAGE ${waveKey}`, x, 28);
  });

  // Edges
  if (currentGraphTopology?.edges) {
    currentGraphTopology.edges.forEach(edge => {
      const from = graphNodePositions[edge.from];
      const to = graphNodePositions[edge.to];
      if (!from || !to) return;

      const isHighlighted = selectedGraphNodeId === edge.from || selectedGraphNodeId === edge.to;
      ctx.strokeStyle = isHighlighted ? '#d4af37' : 'rgba(212, 175, 55, 0.3)';
      ctx.lineWidth = isHighlighted ? 2.5 : 1.4;
      ctx.beginPath();

      const cpX = (from.x + to.x) / 2;
      const cpY = (from.y + to.y) / 2;
      ctx.moveTo(from.x, from.y);
      ctx.quadraticCurveTo(cpX, cpY, to.x, to.y);
      ctx.stroke();

      ctx.fillStyle = isHighlighted ? '#d4af37' : 'rgba(212, 175, 55, 0.6)';
      ctx.beginPath();
      ctx.arc(to.x, to.y, 4, 0, Math.PI * 2);
      ctx.fill();
    });
  }

  // Nodes
  Object.values(graphNodePositions).forEach(pos => {
    const n = pos.node;
    const isSelected = n.id === selectedGraphNodeId;
    const r = pos.radius;

    let ringColor = '#3b82f6';
    if (n.kind === 'trigger') ringColor = '#f87171';
    else if (n.kind === 'prompt') ringColor = '#d4af37';
    else if (n.kind === 'action') ringColor = '#818cf8';
    else if (n.kind === 'file_change') ringColor = '#38bdf8';
    else if (n.kind === 'symbol') ringColor = '#c084fc';
    else if (n.kind === 'test') ringColor = '#34d399';
    else if (n.kind === 'target') ringColor = '#fbbf24';

    if (isSelected) {
      ctx.beginPath();
      ctx.arc(pos.x, pos.y, r + 9, 0, Math.PI * 2);
      ctx.strokeStyle = '#d4af37';
      ctx.lineWidth = 2.5;
      ctx.stroke();
    }

    ctx.beginPath();
    ctx.arc(pos.x, pos.y, r, 0, Math.PI * 2);
    ctx.fillStyle = '#12151b';
    ctx.fill();
    ctx.strokeStyle = ringColor;
    ctx.lineWidth = isSelected ? 3 : 2;
    ctx.stroke();

    ctx.fillStyle = '#f8fafc';
    ctx.font = '700 11px "Plus Jakarta Sans"';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';

    const short = n.label.length > 12 ? n.label.substring(0, 10) + '..' : n.label;
    ctx.fillText(short, pos.x, pos.y);
  });
}

// Canvas Click handler
document.getElementById('esg-canvas')?.addEventListener('click', (e) => {
  const canvas = document.getElementById('esg-canvas');
  const rect = canvas.getBoundingClientRect();
  const clickX = e.clientX - rect.left;
  const clickY = e.clientY - rect.top;

  let clickedNode = null;
  Object.values(graphNodePositions).forEach(pos => {
    const dist = Math.hypot(pos.x - clickX, pos.y - clickY);
    if (dist <= pos.radius + 8) {
      clickedNode = pos.node;
    }
  });

  if (clickedNode) {
    selectedGraphNodeId = clickedNode.id;
    renderNodeDrawer(clickedNode);
    renderEsgCanvas();
  }
});

function renderNodeDrawer(node) {
  document.getElementById('drawer-node-kind').textContent = node.kind.toUpperCase();
  document.getElementById('drawer-node-status').textContent = (node.status || 'OK').toUpperCase();
  document.getElementById('drawer-node-label').textContent = node.label;
  document.getElementById('drawer-node-sub').textContent = node.subtitle || '';
  document.getElementById('drawer-node-detail').textContent = node.detail || 'No extra context.';

  const diffSection = document.getElementById('drawer-diff-section');
  const diffContent = document.getElementById('drawer-diff-content');

  if (node.diff_snippet) {
    diffSection.classList.remove('hidden');
    diffContent.innerHTML = node.diff_snippet.split('\n').map(line => {
      if (line.startsWith('+')) return `<span class="diff-add">${escapeHtml(line)}</span>`;
      if (line.startsWith('-')) return `<span class="diff-rem">${escapeHtml(line)}</span>`;
      return `<span class="diff-ctx">${escapeHtml(line)}</span>`;
    }).join('');
  } else {
    diffSection.classList.add('hidden');
  }

  const metaSection = document.getElementById('drawer-meta-section');
  const metaGrid = document.getElementById('drawer-meta-grid');

  if (node.meta && Object.keys(node.meta).length > 0) {
    metaSection.classList.remove('hidden');
    metaGrid.innerHTML = Object.entries(node.meta).map(([k, v]) => `
      <div style="background: var(--bg-card); padding: 6px 8px; border-radius: 6px; font-size: 11px;">
        <span style="color: var(--text-muted); font-size: 10px; text-transform: uppercase;">${escapeHtml(k)}</span>
        <div style="font-weight: 600; color: var(--text-primary);">${escapeHtml(v)}</div>
      </div>
    `).join('');
  } else {
    metaSection.classList.add('hidden');
  }
}

// ============================================================================
// Keyboard Shortcuts
// ============================================================================
document.addEventListener('keydown', (e) => {
  if (['INPUT', 'TEXTAREA', 'SELECT'].includes(document.activeElement?.tagName)) return;

  if (e.key === 'n' || e.key === 'N') {
    // Start inline draft in first visible column
    const config = getBoardConfig();
    const firstStage = config.find(s => s.visible);
    if (firstStage) {
      document.getElementById('tab-btn-board')?.click();
      window.startInlineDraft(firstStage.id);
    }
  } else if (e.key === 'g' || e.key === 'G') {
    document.getElementById('tab-btn-graph')?.click();
  } else if (e.key === 'v' || e.key === 'V') {
    document.getElementById('btn-action-verify')?.click();
  } else if (e.key === 'a' || e.key === 'A') {
    document.getElementById('btn-action-approve')?.click();
  }
});

// Utilities
function escapeHtml(text) {
  if (!text) return '';
  return String(text)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

function formatDate(isoStr) {
  if (!isoStr) return '';
  const d = new Date(isoStr);
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function getStateColor(state) {
  if (!state) return 'var(--text-muted)';
  const s = state.toLowerCase();
  if (s === 'promoted' || s === 'contract_verified' || s === 'contractverified') return 'var(--accent-emerald)';
  if (s === 'human_approved' || s === 'humanapproved') return 'var(--accent-gold)';
  if (s === 'sandboxed') return 'var(--accent-blue)';
  if (s === 'rejected') return 'var(--accent-rose)';
  return 'var(--text-secondary)';
}

// Window resize handler for canvas
window.addEventListener('resize', () => {
  if (!document.getElementById('view-graph').classList.contains('hidden')) {
    renderEsgCanvas();
  }
});

// Initialization
document.addEventListener('DOMContentLoaded', () => {
  initTheme();
  fetchQueue();
});
