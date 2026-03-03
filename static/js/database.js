// static/js/database.js

import { autostart } from '../../pkg/sqlite_wasm.js';
import { 
    db, setDb, currentDatabase, setCurrentDatabase, 
    availableDatabases, setAvailableDatabases,
    setCurrentTable, elements, pageSize,  // ← ADICIONA pageSize AQUI!
    setCurrentPage, setTotalRows, setLastResults, setLastQuery
} from './state.js';
import { 
    updateTreeTables, updateTreeViews, updateTreeTriggers,
    showNoDatabases, showNoTables, showNoResults,
    updateRunButtonState, updateDatabaseSelector, 
    populateDatabaseDropdown, renderTable, updatePagination
} from './ui.js';

// No início do database.js, após os imports
let workerInitialized = false;

async function ensureWorker() {
    if (!workerInitialized) {
        const { initialize_worker } = await import('../../pkg/sqlite_wasm.js');
        await initialize_worker('/sqlite.org/sqlite3-worker1.js');
        workerInitialized = true;
    }
}

export async function refreshDatabases() {
    try {
        // 🔥 ESCANEIA O OPFS
        const databases = await scanOPFSDatabases();
        setAvailableDatabases(databases);
        
        updateDatabaseSelector();
        await populateDatabaseDropdown();
        
        // ✅ NÃO SELECIONA MAIS AUTOMATICAMENTE
        setCurrentDatabase(null);
        setDb(null);
        showNoDatabases();
        
        updateRunButtonState();
        
    } catch (error) {
        console.error('Failed to refresh databases:', error);
        showNoDatabases();
    }
}

export async function createNewDatabase(dbName) {
    console.log('📦 Creating new database:', dbName);
    
    try {
        const { autostart } = await import('../../pkg/sqlite_wasm.js');
        const { setDb, setCurrentDatabase, availableDatabases, setAvailableDatabases } = await import('./state.js');
        
        console.log('1️⃣ Calling autostart...');
        const newDb = await autostart('/sqlite.org/sqlite3-worker1.js', dbName);
        console.log('2️⃣ Autostart returned:', newDb ? 'OK' : 'null');
        
        if (!newDb) {
            console.error('❌ autostart returned null');
            return null;
        }
        
        console.log('3️⃣ Setting db...');
        setDb(newDb);
        
        console.log('4️⃣ Setting current database...');
        setCurrentDatabase(dbName);
        
        console.log('5️⃣ Updating available databases...');
        if (!availableDatabases.includes(dbName)) {
            setAvailableDatabases([...availableDatabases, dbName]);
        }
        
        console.log('6️⃣ Updating UI...');
        updateDatabaseSelector();
        await populateDatabaseDropdown();
        
        console.log('7️⃣ Creating sample table...');
        await newDb.query(
            "CREATE TABLE IF NOT EXISTS sample (id INTEGER PRIMARY KEY, name TEXT)",
            []
        );
        
        console.log('8️⃣ Loading schema...');
        await loadDatabaseSchema();
        
        console.log('9️⃣ Updating run button...');
        updateRunButtonState();
        
        console.log('✅ Database created successfully:', dbName);
        return newDb;
        
    } catch (error) {
        console.error('❌ Failed to create database:', error);
        console.error('Error details:', {
            message: error.message,
            stack: error.stack,
            error: error
        });
        alert(`Error creating database: ${error.message || error}`);
        return null;
    }
}

async function scanOPFSDatabases() {
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
                // 🔥 IGNORA OS BANCOS TEMPORÁRIOS
                if (fileName !== '_temp_.db' && 
                    fileName !== '___worker_.db' && 
                    (fileName.endsWith('.sqlite3') || fileName.endsWith('.db'))) {
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
        // Tenta carregar as tabelas
        let tables = [];
        try {
            const tablesResult = await currentDb.query(
                "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
                []
            );
            tables = tablesResult.result?.resultRows || [];
        } catch (e) {
            console.log('No tables found or error:', e);
            tables = [];
        }
        
        updateTreeTables(tables);
        updateTreeViews([]); // Simplificado
        updateTreeTriggers([]); // Simplificado
        
        if (tables.length > 0) {
            const firstTable = tables[0].name || tables[0];
            setCurrentTable(firstTable);
            await loadTableData(firstTable);
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
        setCurrentPage(page);                                                                                 
        const offset = (page - 1) * pageSize;  // ← pageSize AGORA É NÚMERO!
        
        console.log('📊 Loading table:', {
            tableName,
            page,
            pageSize,  // ← Agora é número
            offset
        });
        
        elements.resultsHeader.innerHTML = `<i class="fas fa-spinner fa-spin"></i> Loading...`;
        
        const sql = `SELECT * FROM ${tableName} LIMIT ${pageSize} OFFSET ${offset}`;
        console.log('📝 SQL:', sql);
        
        const result = await currentDb.query(sql, []);              
        const rows = result.result?.resultRows || [];
        setLastResults(rows);
        
        const countResult = await currentDb.query(
            `SELECT COUNT(*) as count FROM ${tableName}`,
            []               
        );              
        const total = countResult.result?.resultRows[0]?.count || 0;
        setTotalRows(total);                                                                                  
        
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
            upperSql.includes('ALTER TABLE')) {
            await loadDatabaseSchema();
        }
        
    } catch (error) {
        console.error('Query failed:', error);
        if (elements.tableWrapper) {
            elements.tableWrapper.innerHTML = `
                <div class="error-state">
                    <i class="fas fa-exclamation-triangle" style="font-size: 2rem; color: var(--error); margin-bottom: 1rem;"></i>
                    <h3>Query Error</h3>
                    <p>${error.message || error}</p>
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