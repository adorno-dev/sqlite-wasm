// static/js/studio.js

import init from '../../pkg/sqlite_wasm.js';
import { 
    elements, setAvailableDatabases, setDb, setCurrentDatabase,
    setPageSize, setCurrentPage, currentTable, lastResults, pageSize,
    availableDatabases
} from './state.js';
import { 
    refreshDatabases, runQuery, loadTableData
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
        
        const { autostart } = await import('../../pkg/sqlite_wasm.js');
        const { setDb, setCurrentDatabase, setAvailableDatabases } = await import('./state.js');
        
        // 🔥 RESETA TUDO
        setDb(null);
        setCurrentDatabase(null);
        setAvailableDatabases([]);
        
        // 🔥 CRIA UM WORKER NOVO COM BANCO FIXO
        const workerDb = await autostart('/sqlite.org/sqlite3-worker1.js', '___worker_.db');
        setDb(workerDb);
        
        await refreshDatabases();
        
        if (availableDatabases.length === 0) {
            showNoDatabases();
        }
        
        setTimeout(() => {
            updateRunButtonState();
            console.log('✅ Initial run button state updated');
        }, 100);
        
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
    
    // Close dropdown when clicking outside
    document.addEventListener('click', (e) => {
        if (!elements.dbSelector?.contains(e.target) && elements.dbDropdown) {
            elements.dbDropdown.classList.remove('show');
        }
    });
    
    // Refresh button
    if (elements.refreshBtn) {
        elements.refreshBtn.addEventListener('click', refreshDatabases);
    }
    
    // New Database button
    const newDbBtn = document.getElementById('newDbBtn');
    if (newDbBtn) {
        newDbBtn.addEventListener('click', async () => {
            const dbName = prompt('Enter database name:', 'mydb.db');
            if (dbName && dbName.trim()) {
                const { createNewDatabase } = await import('./database.js');
                await createNewDatabase(dbName.trim());
                const { populateDatabaseDropdown } = await import('./ui.js');
                await populateDatabaseDropdown();
            }
        });
    }
    
    // ===== EDITOR =====
    if (elements.sqlEditor) {
        console.log('📝 Setting up editor events');
        
        elements.sqlEditor.addEventListener('change', () => {
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
    if (elements.pageSizeSelect) {
        elements.pageSizeSelect.addEventListener('change', async () => {
            const newSize = parseInt(elements.pageSizeSelect.value);
            setPageSize(newSize);
            setCurrentPage(1);
            
            if (currentTable) {
                await loadTableData(currentTable);
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
                icon.className = document.body.classList.contains('light') ? 'fas fa-sun' : 'fas fa-moon';
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