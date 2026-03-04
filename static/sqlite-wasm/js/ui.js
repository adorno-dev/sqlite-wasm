// static/js/ui.js

import {
    elements, db, currentDatabase, availableDatabases,
    currentTable, currentPage, pageSize, totalRows, lastResults,
    setCurrentTable, setCurrentPage, setLastResults, setTotalRows
} from './state.js';
import { loadTableData, createNewDatabase } from './database.js';
import { toggleDeleteButton } from './studio.js'

// ===== DROPDOWN FUNCTIONS =====
export function toggleDropdown(e) {
    e.stopPropagation();
    if (elements.dbDropdown) {
        elements.dbDropdown.classList.toggle('show');
    }
}

export function updateDatabaseSelector() {
    if (!elements.dbSelectorSpan) return;

    if (currentDatabase) {
        elements.dbSelectorSpan.textContent = currentDatabase;
        elements.dbSelectorSpan.style.color = '';
        elements.dbSelectorSpan.style.fontStyle = '';
    } else {
        elements.dbSelectorSpan.textContent = 'Choose a database';
        elements.dbSelectorSpan.style.color = 'var(--text-muted)';
        elements.dbSelectorSpan.style.fontStyle = 'italic';
    }
}

export async function populateDatabaseDropdown() {
    const dropdown = elements.dbDropdown;
    if (!dropdown) return;

    if (!dropdown._listenerAdded) {
        dropdown.addEventListener('click', async function handler(e) {
            const option = e.target.closest('.db-option');
            if (!option) return;

            const dbName = option.getAttribute('data-db');
            if (dbName === undefined) return;

            e.stopPropagation();
            e.preventDefault();

            dropdown.classList.remove('show');

            if (dbName === 'null') {
                if (currentDatabase !== null) {
                    await switchDatabase(null);
                }
            } else if (dbName !== currentDatabase) {
                await switchDatabase(dbName);
            }
        });
        dropdown._listenerAdded = true;
    }

    const wasOpen = dropdown.classList.contains('show');
    dropdown.innerHTML = '';

    if (currentDatabase) {
        const chooseOption = document.createElement('div');
        chooseOption.className = 'db-option choose-db';
        chooseOption.innerHTML = `<i class="fas fa-undo"></i> Choose a database`;
        chooseOption.setAttribute('data-db', 'null');
        dropdown.appendChild(chooseOption);

        const separator = document.createElement('div');
        separator.className = 'db-separator';
        separator.innerHTML = '<hr>';
        dropdown.appendChild(separator);
    }

    if (availableDatabases.length > 0) {
        [...new Set(availableDatabases)].forEach(dbName => {
            const option = document.createElement('div');
            option.className = 'db-option';
            if (dbName === currentDatabase) {
                option.classList.add('active');
            }
            option.innerHTML = `
                <i class="fas fa-database"></i>
                ${dbName}
                ${dbName === currentDatabase ? '<span class="db-size">(current)</span>' : ''}
            `;
            option.setAttribute('data-db', dbName);
            dropdown.appendChild(option);
        });
    } else {
        const emptyOption = document.createElement('div');
        emptyOption.className = 'db-option empty-section';
        emptyOption.innerHTML = `<i class="fas fa-database"></i> No databases found`;
        dropdown.appendChild(emptyOption);
    }

    if (wasOpen) dropdown.classList.add('show');
}

export async function switchDatabase(dbName) {

    const { setCurrentDatabase, setCurrentTable } = await import('./state.js');
    const { loadDatabaseSchema } = await import('./database.js');
    const { open } = await import('/sqlite-wasm.js');

    setCurrentDatabase(dbName);
    toggleDeleteButton(!!dbName);

    if (dbName === null) {
        setCurrentTable(null);
        setLastResults([]);
        setTotalRows(0);
        showNoDatabases();
        updateTreeTables([]);
        updateTreeViews([]);
        updateTreeTriggers([]);
        updatePagination();
    } else {
        setCurrentTable(null);
        await open(dbName);

        await loadDatabaseSchema();
    }

    updateRunButtonState();
    updateDatabaseSelector();
    await populateDatabaseDropdown();
}


// ===== TREE UPDATES =====
export function updateTreeTables(tables) {
    if (!elements.treeTables) return;

    if (tables.length === 0) {
        elements.treeTables.innerHTML = `
            <div class="tree-item empty-section">
                <i class="fas fa-table"></i>
                <span class="empty-text">[no tables]</span>
            </div>
        `;
        return;
    }

    let html = '';
    tables.forEach(table => {
        const tableName = table.name || table;
        html += `
            <div class="tree-item" data-table="${tableName}">
                <i class="fas fa-table"></i>
                ${tableName}
            </div>
        `;
    });

    elements.treeTables.innerHTML = html;

    if (currentTable) {
        const activeItem = document.querySelector(`.tree-item[data-table="${currentTable}"]`);
        if (activeItem) {
            activeItem.classList.add('active');
        }
    }

    document.querySelectorAll('.tree-item[data-table]').forEach(item => {
        item.addEventListener('click', () => {
            document.querySelectorAll('.tree-item[data-table], .tree-item[data-view], .tree-item[data-trigger]').forEach(el => {
                el.classList.remove('active');
            });

            const table = item.dataset.table;
            setCurrentTable(table);

            localStorage.setItem('sqlite-studio-last-item', JSON.stringify({
                type: 'table',
                name: table
            }));

            // 🔥 RESETA A PÁGINA PARA 1
            localStorage.setItem('sqlite-studio-current-page', '1');
            setCurrentPage(1);

            loadTableData(table, 1);  // ← PASSA PÁGINA 1
            item.classList.add('active');
        });
    });
}

export function updateTreeViews(views) {
    if (!elements.treeViews) return;

    if (views.length === 0) {
        elements.treeViews.innerHTML = `
            <div class="tree-item empty-section">
                <i class="fas fa-eye"></i>
                <span class="empty-text">[no views]</span>
            </div>
        `;
        return;
    }

    let html = '';
    views.forEach(view => {
        const viewName = view.name || view;
        html += `
            <div class="tree-item" data-view="${viewName}">
                <i class="fas fa-eye"></i>
                ${viewName}
            </div>
        `;
    });

    elements.treeViews.innerHTML = html;

    document.querySelectorAll('.tree-item[data-view]').forEach(item => {
        item.addEventListener('click', () => {
            document.querySelectorAll('.tree-item[data-table], .tree-item[data-view], .tree-item[data-trigger]').forEach(el => {
                el.classList.remove('active');
            });

            const viewName = item.dataset.view;

            localStorage.setItem('sqlite-studio-last-item', JSON.stringify({
                type: 'view',
                name: viewName
            }));

            // 🔥 RESETA A PÁGINA PARA 1
            localStorage.setItem('sqlite-studio-current-page', '1');
            setCurrentPage(1);

            item.classList.add('active');
            loadViewData(viewName, 1);  // ← PASSA PÁGINA 1
        });
    });
}

// 🔥 MODIFICADA PARA RECEBER PÁGINA
async function loadViewData(viewName, page = 1) {
    const { db, setCurrentView, setTotalRows, pageSize } = await import('./state.js');
    const { setCurrentTable } = await import('./state.js');
    const { updatePagination } = await import('./ui.js');

    if (!db) return;

    try {
        setCurrentTable(null);
        setCurrentView(viewName);
        setTotalRows(0);

        elements.resultsHeader.innerHTML = `<i class="fas fa-spinner fa-spin"></i> Loading view...`;

        // 🔥 USA A PÁGINA RECEBIDA
        const offset = (page - 1) * pageSize;
        const result = await db.query(`SELECT * FROM ${viewName} LIMIT ${pageSize} OFFSET ${offset}`, []);
        const rows = result.result?.resultRows || [];

        // Tenta contar total (opcional)
        let total = rows.length;
        try {
            const countResult = await db.query(`SELECT COUNT(*) as count FROM ${viewName}`, []);
            total = countResult.result?.resultRows[0]?.count || rows.length;
        } catch (e) { }

        if (rows.length === 0) {
            showNoResults();
            setTotalRows(0);
        } else {
            renderTable(rows);
            elements.resultsHeader.innerHTML = `<i class="fas fa-eye"></i> View: ${viewName} · ${total} rows`;
            setTotalRows(total);
        }

        updatePagination();

        const { markActiveTreeItem } = await import('./ui.js');
        markActiveTreeItem('view', viewName);

    } catch (error) {
        console.error('Failed to load view:', error);
        showNoResults();
        setTotalRows(0);
        updatePagination();
    }
}

export function updateTreeTriggers(triggers) {
    if (!elements.treeTriggers) return;

    if (triggers.length === 0) {
        elements.treeTriggers.innerHTML = `
            <div class="tree-item empty-section">
                <i class="fas fa-bolt"></i>
                <span class="empty-text">[no triggers]</span>
            </div>
        `;
        return;
    }

    let html = '';
    triggers.forEach(trigger => {
        const triggerName = trigger.name || trigger;
        html += `
            <div class="tree-item" data-trigger="${triggerName}">
                <i class="fas fa-bolt"></i>
                ${triggerName}
            </div>
        `;
    });

    elements.treeTriggers.innerHTML = html;

    document.querySelectorAll('.tree-item[data-trigger]').forEach(item => {
        item.addEventListener('click', () => {
            document.querySelectorAll('.tree-item[data-table], .tree-item[data-view], .tree-item[data-trigger]').forEach(el => {
                el.classList.remove('active');
            });

            const triggerName = item.dataset.trigger;

            localStorage.setItem('sqlite-studio-last-item', JSON.stringify({
                type: 'trigger',
                name: triggerName
            }));

            item.classList.add('active');
            elements.resultsHeader.innerHTML = `<i class="fas fa-bolt"></i> Trigger: ${triggerName}`;
        });
    });
}

// ===== UI STATE MESSAGES =====
export function showNoDatabases() {
    if (elements.resultsHeader) {
        elements.resultsHeader.innerHTML = `<i class="fas fa-database"></i> No Database`;
    }

    if (elements.tableWrapper) {
        elements.tableWrapper.innerHTML = `
            <div class="empty-state">
                <i class="fas fa-database" style="font-size: 3rem; color: var(--warning); margin-bottom: 1rem;"></i>
                <h3>No Database Selected</h3>
                <p>Select a database from the dropdown or create a new one.</p>
                <p style="margin-top: 1rem; font-size: 0.85rem; color: var(--text-muted);">
                    <i class="fas fa-info-circle"></i> 
                    Use the "New Database" button in the dropdown to create one.
                </p>
            </div>
        `;
    }

    updateTreeTables([]);
    updateTreeViews([]);
    updateTreeTriggers([]);

    import('./state.js').then(({ setTotalRows }) => {
        setTotalRows(0);
        updatePagination();
    });

    if (elements.paginationInfo) {
        elements.paginationInfo.textContent = '0 rows';
    }
}

export function showNoTables() {
    updateTreeTables([]);

    if (elements.resultsHeader) {
        elements.resultsHeader.innerHTML = `<i class="fas fa-table"></i> No Table Selected`;
    }

    if (elements.tableWrapper) {
        elements.tableWrapper.innerHTML = `
            <div class="empty-state">
                <i class="fas fa-table" style="font-size: 3rem; color: var(--warning); margin-bottom: 1rem;"></i>
                <h3>No Tables Found</h3>
                <p>This database has no tables yet.</p>
                <p style="margin-top: 1rem; font-size: 0.85rem; color: var(--text-muted);">
                    <i class="fas fa-info-circle"></i> 
                    Run a CREATE TABLE command to create one.
                </p>
            </div>
        `;
    }

    if (elements.paginationInfo) {
        elements.paginationInfo.textContent = '0 rows';
    }
}

export function showNoResults() {
    if (elements.tableWrapper) {
        elements.tableWrapper.innerHTML = `
            <div class="empty-state">
                <i class="fas fa-search" style="font-size: 3rem; color: var(--text-muted); margin-bottom: 1rem;"></i>
                <h3>No Results</h3>
                <p>Your query returned no rows.</p>
            </div>
        `;
    }

    if (elements.resultsHeader) {
        elements.resultsHeader.innerHTML = `<i class="fas fa-table"></i> No Results`;
    }

    if (elements.paginationInfo) {
        elements.paginationInfo.textContent = '0 rows';
    }
}

// ===== RUN BUTTON STATE =====
export function updateRunButtonState() {
    if (!elements.runBtn) return;

    const sqlEditor = elements.sqlEditor;
    if (!sqlEditor) return;

    import('./state.js').then(({ db, currentDatabase }) => {
        const hasText = sqlEditor.value.trim().length > 0;
        const hasDatabase = currentDatabase !== null;

        elements.runBtn.disabled = !(hasText && hasDatabase);
    });
}

// ===== UI RENDERING =====
export function renderTable(rows) {
    if (!rows || rows.length === 0 || !elements.tableWrapper) {
        showNoResults();
        return;
    }

    const columns = Object.keys(rows[0]);

    let html = '<table class="data-table"><thead><tr>';
    columns.forEach(col => {
        html += `<th>${col}</th>`;
    });
    html += '</tr></thead><tbody>';

    rows.forEach(row => {
        html += '<tr>';
        columns.forEach(col => {
            html += `<td>${formatValue(row[col])}</td>`;
        });
        html += '</tr>';
    });

    html += '</tbody></table>';

    elements.tableWrapper.innerHTML = html;
}

function formatValue(value) {
    if (value === null || value === undefined) return '<span class="null-value">NULL</span>';
    if (typeof value === 'string') return escapeHtml(value);
    return String(value);
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// ===== PAGINATION =====
export function updatePagination() {
    if (!elements.paginationInfo) return;

    const totalPages = Math.ceil(totalRows / pageSize);
    const start = ((currentPage - 1) * pageSize) + 1;
    const end = Math.min(currentPage * pageSize, totalRows);

    if (totalRows === 0) {
        elements.paginationInfo.textContent = '0 rows';
    } else {
        elements.paginationInfo.textContent = `Showing ${start}-${end} of ${totalRows} rows`;
    }

    let pageNumbersHtml = '';
    if (totalPages > 0) {
        // Calcula o range de páginas a mostrar (sempre 5 páginas ao redor da atual)
        let startPage = Math.max(1, currentPage - 2);
        let endPage = Math.min(totalPages, startPage + 4);

        // Ajusta se estiver no final
        if (endPage - startPage < 4) {
            startPage = Math.max(1, endPage - 4);
        }

        // Primeira página se não estiver no início
        if (startPage > 1) {
            pageNumbersHtml += `<button class="btn-small page-number" data-page="1">1</button>`;
            if (startPage > 2) {
                pageNumbersHtml += `<span class="page-separator">...</span>`;
            }
        }

        // Páginas do range
        for (let i = startPage; i <= endPage; i++) {
            const activeClass = i === currentPage ? 'active' : '';
            pageNumbersHtml += `<button class="btn-small page-number ${activeClass}" data-page="${i}">${i}</button>`;
        }

        // Última página se não estiver no final
        if (endPage < totalPages) {
            if (endPage < totalPages - 1) {
                pageNumbersHtml += `<span class="page-separator">...</span>`;
            }
            pageNumbersHtml += `<button class="btn-small page-number" data-page="${totalPages}">${totalPages}</button>`;
        }
    }
    elements.pageNumbers.innerHTML = pageNumbersHtml;

    // Remove event listeners antigos e adiciona novos
    document.querySelectorAll('.page-number').forEach(btn => {
        btn.addEventListener('click', () => {
            const page = parseInt(btn.dataset.page);
            setCurrentPage(page);
            if (currentTable) {
                loadTableData(currentTable, page);
            } else if (lastResults.length > 0) {
                const start = (page - 1) * pageSize;
                const end = start + pageSize;
                renderTable(lastResults.slice(start, end));
                updatePagination();
            }
        });
    });

    if (elements.prevPageBtn) {
        elements.prevPageBtn.disabled = currentPage === 1;
        elements.prevPageBtn.onclick = () => {
            if (currentPage > 1) {
                setCurrentPage(currentPage - 1);
                if (currentTable) {
                    loadTableData(currentTable, currentPage);
                } else if (lastResults.length > 0) {
                    const start = (currentPage - 1) * pageSize;
                    const end = start + pageSize;
                    renderTable(lastResults.slice(start, end));
                    updatePagination();
                }
            }
        };
    }

    if (elements.nextPageBtn) {
        elements.nextPageBtn.disabled = currentPage === totalPages || totalPages === 0;
        elements.nextPageBtn.onclick = () => {
            if (currentPage < totalPages) {
                setCurrentPage(currentPage + 1);
                if (currentTable) {
                    loadTableData(currentTable, currentPage);
                } else if (lastResults.length > 0) {
                    const start = (currentPage - 1) * pageSize;
                    const end = start + pageSize;
                    renderTable(lastResults.slice(start, end));
                    updatePagination();
                }
            }
        };
    }
}

// ===== EXPORT FUNCTIONS =====
export function exportCSV() {
    if (!lastResults || lastResults.length === 0) return;

    const columns = Object.keys(lastResults[0]);
    const csv = [
        columns.join(','),
        ...lastResults.map(row => columns.map(col => {
            const val = row[col];
            if (val === null || val === undefined) return '';
            const strVal = String(val).replace(/"/g, '""');
            return strVal.includes(',') || strVal.includes('"') || strVal.includes('\n')
                ? `"${strVal}"`
                : strVal;
        }).join(','))
    ].join('\n');

    const blob = new Blob([csv], { type: 'text/csv' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `export_${new Date().toISOString().slice(0, 10)}.csv`;
    a.click();
    URL.revokeObjectURL(url);
}

export function exportJSON() {
    if (!lastResults || lastResults.length === 0) return;

    const json = JSON.stringify(lastResults, null, 2);
    navigator.clipboard.writeText(json).then(() => {
        const originalText = elements.exportJsonBtn.innerHTML;
        elements.exportJsonBtn.innerHTML = '<i class="fas fa-check"></i> Copied!';
        setTimeout(() => {
            elements.exportJsonBtn.innerHTML = originalText;
        }, 2000);
    });
}

export function markActiveTreeItem(type, name) {
    document.querySelectorAll('.tree-item[data-table], .tree-item[data-view], .tree-item[data-trigger]').forEach(el => {
        el.classList.remove('active');
    });

    const selector = `.tree-item[data-${type}="${name}"]`;
    const activeItem = document.querySelector(selector);
    if (activeItem) {
        activeItem.classList.add('active');
    }
}

window.updateRunButtonState = updateRunButtonState;
