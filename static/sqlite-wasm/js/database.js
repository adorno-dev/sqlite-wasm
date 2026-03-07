// static/js/database.js

import * as wasm from '/sqlite-wasm.js';
const { initialize_worker, open, close } = wasm;

import { 
    db, setDb, currentDatabase, setCurrentDatabase, 
    availableDatabases, setAvailableDatabases,
    setCurrentTable, elements, pageSize,
    setCurrentPage, setTotalRows, setLastResults, setLastQuery
} from './state.js';
import { 
    updateTreeTables, updateTreeViews, updateTreeTriggers,
    showNoDatabases, showNoTables, showNoResults,
    updateRunButtonState, updateDatabaseSelector, 
    populateDatabaseDropdown, renderTable, updatePagination
} from './ui.js';
import { toggleDeleteButton } from './studio.js'

let workerInitialized = false;

async function ensureWorker() {
    if (!workerInitialized) {
        await initialize_worker('/sqlite.org/sqlite3-worker1.js');
        workerInitialized = true;
    }
}

export async function createNewDatabase(dbName) {
    
    try {
        await open(dbName);
        
        setCurrentDatabase(dbName);
        toggleDeleteButton(!!dbName);
        
        if (!availableDatabases.includes(dbName)) {
            setAvailableDatabases([...availableDatabases, dbName]);
        }
        
        updateDatabaseSelector();
        await populateDatabaseDropdown();
        
        await loadDatabaseSchema();
        
        updateRunButtonState();
        
    } catch (error) {
        console.error('❌ Failed to create database:', error);
        alert(`Error creating database: ${error.message || error}`);
    }
}

export async function scanOPFSDatabases() {
    try {
        const databases = [];
        
        const root = await navigator.storage.getDirectory();
        const entries = [];
        for await (const entry of root.values()) {
            entries.push(entry);
        }
        
        for (const entry of entries) {
            if (entry.kind === 'file') {
                const fileName = entry.name;
                if (fileName.endsWith('.sqlite3') || fileName.endsWith('.db')) {
                    databases.push(fileName);
                }
            }
        }
        
        return databases;
        
    } catch (error) {
        console.error('OPFS scan failed:', error);
        return [];
    }
}

export async function loadDatabaseSchema() {
    const currentDb = db;
    if (!currentDb) return;
    
    try {
        let tables = [];
        try {
            const tablesResult = await currentDb.query(
                "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
                []
            );
            tables = tablesResult.result?.resultRows || [];
        } catch (e) {
            tables = [];
        }
        updateTreeTables(tables);
        
        let views = [];
        try {
            const viewsResult = await currentDb.query(
                "SELECT name FROM sqlite_master WHERE type='view' ORDER BY name",
                []
            );
            views = viewsResult.result?.resultRows || [];
        } catch (e) {
            views = [];
        }
        updateTreeViews(views);
        
        let triggers = [];
        try {
            const triggersResult = await currentDb.query(
                "SELECT name FROM sqlite_master WHERE type='trigger' ORDER BY name",
                []
            );
            triggers = triggersResult.result?.resultRows || [];
        } catch (e) {
            triggers = [];
        }
        updateTreeTriggers(triggers);
        
        if (tables.length > 0) {
            const firstTable = tables[0].name || tables[0];
            setCurrentTable(firstTable);
            await loadTableData(firstTable);
            const { markActiveTreeItem } = await import('./ui.js');
            markActiveTreeItem('table', firstTable);
        } else {
            showNoTables();
            setCurrentTable(null);
        }
        
    } catch (error) {
        console.error('Failed to load schema:', error);
        showNoTables();
    }
}

export async function loadTableData(tableName, page = 1) {

    const currentDb = db;                
    if (!tableName || !currentDb) {
        showNoTables();   
        return;                                                                     
    }         
    
    try {
        if (page === 1) {
            const savedPage = localStorage.getItem('sqlite-studio-current-page');
            if (savedPage) {
                page = parseInt(savedPage);
            }
        }
        
        setCurrentPage(page);                                                                                 
        const offset = (page - 1) * pageSize;
        
        elements.resultsHeader.innerHTML = `<i class="fas fa-spinner fa-spin"></i> Loading...`;
        
        const sql = `SELECT * FROM ${tableName} LIMIT ${pageSize} OFFSET ${offset}`;
        
        const result = await currentDb.query(sql, []);              
        const rows = result.result?.resultRows || [];
        setLastResults(rows);
        
        const countResult = await currentDb.query(
            `SELECT COUNT(*) as count FROM ${tableName}`,
            []               
        );              
        const total = countResult.result?.resultRows[0]?.count || 0;
        setTotalRows(total);
        
        localStorage.setItem('sqlite-studio-current-page', page.toString());
        
        if (rows.length === 0) {
            showNoResults();
        } else {
            renderTable(rows);
            elements.resultsHeader.innerHTML = `<i class="fas fa-table"></i> ${tableName} · ${total} rows`;
            updatePagination();
        }
        
    } catch (error) {
        console.error('❌ Failed to load table data:', error);
        showNoResults();
    }
}

export async function runQuery() {
    const sql = elements.sqlEditor.value.trim();
    if (!sql) return;
    
    const currentDb = db;
    
    try {
        elements.runBtn.disabled = true;
        elements.runBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> Running...';
        elements.resultsHeader.innerHTML = `<i class="fas fa-spinner fa-spin"></i> Executing query...`;
        
        if (!currentDb) {
            alert('Please select or create a database first. Use the "New Database" button in the dropdown.');
            elements.runBtn.disabled = false;
            elements.runBtn.innerHTML = '<i class="fas fa-play"></i> Run';
            return;
        }
        
        const result = await currentDb.query(sql, []);
        const rows = result.result?.resultRows || [];
        
        setLastResults(rows);
        setLastQuery(sql);
        
        if (rows.length === 0) {
            showNoResults();
        } else {
            renderTable(rows);
            elements.resultsHeader.innerHTML = `<i class="fas fa-table"></i> Query Results · ${rows.length} rows`;
            setTotalRows(rows.length);
            updatePagination();
        }
        
        const upperSql = sql.toUpperCase();
        
        if (upperSql.includes('CREATE TABLE') || 
            upperSql.includes('DROP TABLE') ||
            upperSql.includes('ALTER TABLE') ||
            upperSql.includes('CREATE VIEW') ||
            upperSql.includes('DROP VIEW') ||
            upperSql.includes('CREATE TRIGGER') ||
            upperSql.includes('DROP TRIGGER')) {
            await loadDatabaseSchema();
        }
        
        if (upperSql.includes('INSERT') || 
            upperSql.includes('UPDATE') || 
            upperSql.includes('DELETE')) {
            
            const { currentTable } = await import('./state.js');
            
            if (currentTable) {
                await loadTableData(currentTable, 1);
            }
        }
        
    } catch (error) {
        console.error('Query failed:', error);
        
        let errorMessage = 'Unknown error';
        
        if (error && typeof error === 'object') {
            if (error.result && error.result.message) {
                errorMessage = error.result.message;
            } else if (error.message) {
                errorMessage = error.message;
            } else if (typeof error === 'string') {
                errorMessage = error;
            } else {
                try {
                    errorMessage = JSON.stringify(error, null, 2);
                } catch (e) {
                    errorMessage = String(error);
                }
            }
        }
        
        if (elements.tableWrapper) {
            elements.tableWrapper.innerHTML = `
                <div class="error-state">
                    <i class="fas fa-exclamation-triangle" style="font-size: 3rem; color: var(--error); margin-bottom: 1rem;"></i>
                    <h3 style="color: var(--error); margin-bottom: 1rem;">Query Error</h3>
                    <p style="color: var(--text-primary); background: var(--bg-tertiary); padding: 1rem; border-radius: 4px; font-family: monospace; text-align: left; max-width: 800px; margin: 0 auto; white-space: pre-wrap; word-break: break-word;">${errorMessage}</p>
                </div>
            `;
        }
        if (elements.resultsHeader) {
            elements.resultsHeader.innerHTML = `<i class="fas fa-exclamation-triangle"></i> Query Failed`;
        }
    } finally {
        elements.runBtn.disabled = false;
        elements.runBtn.innerHTML = '<i class="fas fa-play"></i> Run';
        updateRunButtonState();
    }
}
