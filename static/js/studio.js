// static/js/studio.js

import init from '../../pkg/sqlite_wasm.js';
import { 
    elements, setAvailableDatabases, setDb, setCurrentDatabase,
    setPageSize, setCurrentPage, currentTable, lastResults, pageSize,
    availableDatabases
} from './state.js';
import { 
    runQuery, loadTableData, scanOPFSDatabases, createNewDatabase
} from './database.js';
import { 
    toggleDropdown, updateRunButtonState, 
    exportCSV, exportJSON, populateDatabaseDropdown,
    updateDatabaseSelector, showNoDatabases, renderTable, updatePagination
} from './ui.js';

// ===== INITIALIZATION =====
async function initialize() {
    try {
        console.log('🚀 Initializing SQLite Studio...');
        await init();
        
        const { autostart, open } = await import('../../pkg/sqlite_wasm.js');
        const { loadDatabaseSchema, loadTableData } = await import('./database.js');
        const { setCurrentTable, setDb, setCurrentDatabase, setAvailableDatabases } = await import('./state.js');
        const { showNoDatabases, updateDatabaseSelector, populateDatabaseDropdown, updateRunButtonState } = await import('./ui.js');
        
        // 0️⃣ Recupera o tema salvo
        const savedTheme = localStorage.getItem('sqlite-studio-theme');
        if (savedTheme === 'light') {
            document.body.classList.add('light');
            if (elements.themeToggle) {
                const icon = elements.themeToggle.querySelector('i');
                if (icon) icon.className = 'fas fa-sun';
            }
        }
        
        // 1️⃣ Inicializa worker
        console.log('1️⃣ Initializing worker...');
        const workerDb = await autostart('/sqlite.org/sqlite3-worker1.js');
        setDb(workerDb);
        
        // 2️⃣ Escaneia bancos existentes
        console.log('3️⃣ Scanning OPFS...');
        const databases = await scanOPFSDatabases();
        setAvailableDatabases(databases);
        
        // 3️⃣ Recupera banco, item e query selecionados
        const lastDb = localStorage.getItem('sqlite-studio-last-db');
        const lastItem = JSON.parse(localStorage.getItem('sqlite-studio-last-item') || 'null');
        const lastQuery = localStorage.getItem('sqlite-studio-last-query');
        
        console.log('🔍 Last selected database:', lastDb);
        console.log('🔍 Last selected item:', lastItem);
        console.log('🔍 Last query:', lastQuery ? 'yes' : 'no');
        
        // 4️⃣ Restaura query no editor (se houver)
        if (lastQuery && elements.sqlEditor) {
            elements.sqlEditor.value = lastQuery;
        }
        
        // 5️⃣ Se o banco ainda existe, seleciona ele
        if (lastDb && databases.includes(lastDb)) {
            console.log('🔄 Restoring last database:', lastDb);
            await open(lastDb);
            setCurrentDatabase(lastDb);
            await loadDatabaseSchema();
            
            // 6️⃣ Se tinha um item selecionado, restaura baseado no tipo
            if (lastItem) {
                if (lastItem.type === 'table') {
                    console.log('📋 Restoring last table:', lastItem.name);
                    setCurrentTable(lastItem.name);
                    await loadTableData(lastItem.name, 1);
                    
                    // Marca no DOM
                    setTimeout(() => {
                        const activeItem = document.querySelector(`.tree-item[data-table="${lastItem.name}"]`);
                        if (activeItem) {
                            document.querySelectorAll('.tree-item[data-table], .tree-item[data-view], .tree-item[data-trigger]').forEach(el => {
                                el.classList.remove('active');
                            });
                            activeItem.classList.add('active');
                        }
                    }, 200);
                    
                } else if (lastItem.type === 'view') {
                    console.log('👁️ Restoring last view:', lastItem.name);
                    setCurrentTable(null);
                    
                    // Carrega a view diretamente
                    const { db } = await import('./state.js');
                    try {
                        const result = await db.query(`SELECT * FROM ${lastItem.name} LIMIT 100`, []);
                        const rows = result.result?.resultRows || [];
                        
                        if (rows.length === 0) {
                            const { showNoResults } = await import('./ui.js');
                            showNoResults();
                        } else {
                            const { renderTable } = await import('./ui.js');
                            renderTable(rows);
                            elements.resultsHeader.innerHTML = `<i class="fas fa-eye"></i> View: ${lastItem.name} · ${rows.length} rows`;
                        }
                    } catch (error) {
                        console.error('Failed to load view:', error);
                        const { showNoResults } = await import('./ui.js');
                        showNoResults();
                    }
                    
                    // Marca no DOM
                    setTimeout(() => {
                        const activeItem = document.querySelector(`.tree-item[data-view="${lastItem.name}"]`);
                        if (activeItem) {
                            document.querySelectorAll('.tree-item[data-table], .tree-item[data-view], .tree-item[data-trigger]').forEach(el => {
                                el.classList.remove('active');
                            });
                            activeItem.classList.add('active');
                        }
                    }, 200);
                }
            }
        } else {
            // 7️⃣ Se não, mostra mensagem de no database
            setCurrentDatabase(null);
            showNoDatabases();
        }
        
        // 8️⃣ Atualiza dropdown
        updateDatabaseSelector();
        await populateDatabaseDropdown();
        
        // 9️⃣ Atualiza botão Run
        setTimeout(() => {
            updateRunButtonState();
            console.log('✅ Initial run button state updated');
        }, 150);
        
    } catch (error) {
        console.error('❌ Initialization failed:', error);
        showNoDatabases();
    }
}

// ===== EVENT LISTENERS =====
function setupEventListeners() {
    console.log('🎯 Setting up event listeners');
    
    // Database selector
    if (elements.dbSelector) {
        elements.dbSelector.addEventListener('click', toggleDropdown);
    }
    
    document.addEventListener('click', (e) => {
        if (!elements.dbSelector?.contains(e.target) && elements.dbDropdown) {
            elements.dbDropdown.classList.remove('show');
        }
    });
    
    // Refresh button
    if (elements.refreshBtn) {
        elements.refreshBtn.addEventListener('click', async () => {
            console.log('🔄 Refreshing database list...');
            const databases = await scanOPFSDatabases();
            setAvailableDatabases(databases);
            await populateDatabaseDropdown();
        });
    }
    
    // New Database button
    const newDbBtn = document.getElementById('newDbBtn');
    if (newDbBtn) {
        newDbBtn.addEventListener('click', async () => {
            const dbName = prompt('Enter database name:', 'mydb.db');
            if (dbName && dbName.trim()) {
                await createNewDatabase(dbName.trim());
                await populateDatabaseDropdown();
            }
        });
    }
    
    // ===== EDITOR =====
    if (elements.sqlEditor) {
        console.log('📝 Setting up editor events');
        
        elements.sqlEditor.addEventListener('input', () => {
            localStorage.setItem('sqlite-studio-last-query', elements.sqlEditor.value);
            updateRunButtonState();
        });
        
        elements.sqlEditor.addEventListener('keydown', (e) => {
            if (e.ctrlKey && e.key === 'Enter') {
                e.preventDefault();
                console.log('🚀 Ctrl+Enter pressed, running query');
                runQuery();
            }
        });
    }
    
    // Run button
    if (elements.runBtn) {
        elements.runBtn.addEventListener('click', runQuery);
    }
    
    // Clear button
    if (elements.clearBtn) {
        elements.clearBtn.addEventListener('click', () => {
            elements.sqlEditor.value = '';
            localStorage.removeItem('sqlite-studio-last-query');
            updateRunButtonState();
            console.log('🧹 Editor cleared');
        });
    }
    
    // Format button
    if (elements.formatBtn) {
        elements.formatBtn.addEventListener('click', () => {
            const sql = elements.sqlEditor.value;
            const keywords = ['SELECT', 'FROM', 'WHERE', 'INSERT', 'UPDATE', 'DELETE', 'CREATE', 'DROP', 'ALTER', 'TABLE', 'INDEX', 'VIEW'];
            let formatted = sql;
            keywords.forEach(keyword => {
                const regex = new RegExp(keyword, 'gi');
                formatted = formatted.replace(regex, keyword.toUpperCase());
            });
            elements.sqlEditor.value = formatted;
            console.log('🎨 SQL formatted');
        });
    }
    
    // Page size
    const pageSizeSelect = document.getElementById('pageSize');
    if (pageSizeSelect) {
        pageSizeSelect.addEventListener('change', async () => {
            const newSize = parseInt(pageSizeSelect.value);
            setPageSize(newSize);
            setCurrentPage(1);
            
            if (currentTable) {
                await loadTableData(currentTable, 1);
            } else if (lastResults.length > 0) {
                renderTable(lastResults.slice(0, newSize));
                updatePagination();
            }
        });
    }

    // Export buttons
    if (elements.exportCsvBtn) {
        elements.exportCsvBtn.addEventListener('click', exportCSV);
    }
    
    if (elements.exportJsonBtn) {
        elements.exportJsonBtn.addEventListener('click', exportJSON);
    }
    
    // Theme toggle
    if (elements.themeToggle) {
        elements.themeToggle.addEventListener('click', () => {
            document.body.classList.toggle('light');
            const icon = elements.themeToggle.querySelector('i');
            if (icon) {
                const isLight = document.body.classList.contains('light');
                icon.className = isLight ? 'fas fa-sun' : 'fas fa-moon';
                // 💾 SALVA O TEMA
                localStorage.setItem('sqlite-studio-theme', isLight ? 'light' : 'dark');
            }
        });
    }
}

// ===== EXPOSE FUNCTIONS GLOBALMENTE =====
window.updateRunButtonState = updateRunButtonState;
window.runQuery = runQuery;

// ===== START =====
document.addEventListener('DOMContentLoaded', () => {
    console.log('📄 DOM Content Loaded');
    setupEventListeners();
    initialize();
});
