// static/js/ui.js

import { 
    elements, db, currentDatabase, availableDatabases, 
    currentTable, currentPage, pageSize, totalRows, lastResults,
    setCurrentTable, setCurrentPage
} from './state.js';
import { loadTableData, createNewDatabase } from './database.js';

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
            
            console.log('👆 Clicked option with dbName:', dbName, 'currentDatabase:', currentDatabase);
            
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
    
    console.log('📋 Populating dropdown. Current DB:', currentDatabase);
    console.log('📋 Available DBs:', availableDatabases);
    
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

async function switchDatabase(dbName) {
    console.log('🔄 Switching database to:', dbName);
    
    const { setCurrentDatabase } = await import('./state.js');
    const { open } = await import('../../pkg/sqlite_wasm.js');
    
    if (dbName === null) {
        setCurrentDatabase(null);
        localStorage.removeItem('sqlite-studio-last-db');
        localStorage.removeItem('sqlite-studio-last-item');
        updateDatabaseSelector();
        showNoDatabases();
        updateTreeTables([]);
        updateTreeViews([]);
        updateTreeTriggers([]);
    } else {
        await open(dbName);
        setCurrentDatabase(dbName);
        localStorage.setItem('sqlite-studio-last-db', dbName);
        updateDatabaseSelector();
        
        const { loadDatabaseSchema } = await import('./database.js');
        await loadDatabaseSchema();
    }
    
    await populateDatabaseDropdown();
    updateRunButtonState();
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
            
            // 🔥 SALVA O ITEM SELECIONADO
            localStorage.setItem('sqlite-studio-last-item', JSON.stringify({
                type: 'table',
                name: table
            }));
            
            loadTableData(table);
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
            console.log('👁️ Loading view:', viewName);
            
            // 🔥 SALVA O ITEM SELECIONADO
            localStorage.setItem('sqlite-studio-last-item', JSON.stringify({
                type: 'view',
                name: viewName
            }));
            
            item.classList.add('active');
            loadViewData(viewName);
        });
    });
}

async function loadViewData(viewName) {
    const { db } = await import('./state.js');
    if (!db) return;
    
    try {
        elements.resultsHeader.innerHTML = `<i class="fas fa-spinner fa-spin"></i> Loading view...`;
        
        const result = await db.query(`SELECT * FROM ${viewName} LIMIT 100`, []);
        const rows = result.result?.resultRows || [];
        
        if (rows.length === 0) {
            showNoResults();
        } else {
            renderTable(rows);
            elements.resultsHeader.innerHTML = `<i class="fas fa-eye"></i> View: ${viewName} · ${rows.length} rows`;
        }
        
    } catch (error) {
        console.error('Failed to load view:', error);
        showNoResults();
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
            console.log('⚡ Trigger selected:', triggerName);
            
            // 🔥 SALVA O ITEM SELECIONADO
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
        
        console.log('🔘 Run button check:', { 
            hasText, 
            hasDatabase,
            currentDatabase,
            valueLength: sqlEditor.value.length,
            dbExists: !!db,
            disabled: !(hasText && hasDatabase)
        });
        
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
        const maxVisible = Math.min(totalPages, 5);
        for (let i = 1; i <= maxVisible; i++) {
            const activeClass = i === currentPage ? 'active' : '';
            pageNumbersHtml += `<button class="btn-small page-number ${activeClass}" data-page="${i}">${i}</button>`;
        }
    }
    elements.pageNumbers.innerHTML = pageNumbersHtml;
    
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
    a.download = `export_${new Date().toISOString().slice(0,10)}.csv`;
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

// No final do ui.js
window.updateRunButtonState = updateRunButtonState;