// static/js/studio.js
// const { autostart, open, close } = await import('/sqlite-wasm.js');
//

// const wasm = await import('/sqlite-wasm.js');
// import * as wasm from '/sqlite-wasm.js';
// const { autostart_embedded, open, close, worker_url, js_url, wasm_url, proxy_url } = wasm;

import * as wasm from '/sqlite-wasm.js';
const { autostart_embedded, autostart, open, close } = wasm;

import * as wasmModule from '/sqlite-wasm.js';

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

// studio.js - no início, depois dos imports
export function toggleDeleteButton(show) {
    const deleteBtn = document.getElementById('deleteDbBtn');
    if (deleteBtn) {
        if (show) {
            deleteBtn.classList.remove('hidden');
        } else {
            deleteBtn.classList.add('hidden');
        }
    }
};


// ===== INITIALIZATION =====
async function initialize() {
    try {
        const { loadDatabaseSchema, loadTableData } = await import('./database.js');
        const { setCurrentTable, setCurrentView, setDb, setCurrentDatabase, setAvailableDatabases, setTotalRows, setCurrentPage, pageSize } = await import('./state.js');
        const {
            showNoDatabases,
            updateDatabaseSelector,
            populateDatabaseDropdown,
            updateRunButtonState,
            markActiveTreeItem,
            showNoResults,
            renderTable,
            updateTreeViews,
            updateTreeTriggers,
            updateTreeTables,
            updatePagination
        } = await import('./ui.js');

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
        // const workerDb = await autostart('/static/sqlite.org/sqlite3-worker1.js');
        // setDb(workerDb);


        // Firefox precisa de instanciação explícita
        if (navigator.userAgent.includes('Firefox')) {
            if (wasmModule.default) {
                await wasmModule.default();
            }
        }

        // Agora pega as funções (já instanciadas)
        const wasm = wasmModule;
        const { autostart_embedded, open, close } = wasm;

        console.log('✅ WASM carregado:', { 
            autostart_embedded: !!autostart_embedded,
            open: !!open,
            close: !!close 
        });

        try {
            const workerDb = await autostart_embedded();
            setDb(workerDb);
        } catch (e) {
            console.error('❌ Erro ao inicializar:', e);
        }


        // // 🔥 USA O OBJETO CERTO!
        // const wasm = wasmModule.wasmBindings || wasmModule;
        //
        // const { autostart_embedded, open, close } = wasm;
        //
        // console.log('✅ WASM carregado:', {
        //     autostart_embedded: !!autostart_embedded,
        //     open: !!open,
        //     close: !!close
        // });
        //
        // const workerDb = await autostart_embedded();
        // setDb(workerDb);


        // 2️⃣ Escaneia bancos existentes
        const databases = await scanOPFSDatabases();
        setAvailableDatabases(databases);

        // 3️⃣ Recupera banco, item e query selecionados
        const lastDb = localStorage.getItem('sqlite-studio-last-db');
        const lastItem = JSON.parse(localStorage.getItem('sqlite-studio-last-item') || 'null');
        const lastQuery = localStorage.getItem('sqlite-studio-last-query');

        // 4️⃣ Restaura query no editor (se houver)
        if (lastQuery && elements.sqlEditor) {
            elements.sqlEditor.value = lastQuery;
        }

        // 5️⃣ Se o banco ainda existe, seleciona ele
        if (lastDb && databases.includes(lastDb)) {
            await open(lastDb);
            setCurrentDatabase(lastDb);

            // Carrega schema completo (tabelas, views, triggers)
            await loadDatabaseSchema();

            // 6️⃣ Se tinha um item selecionado, restaura baseado no tipo
            if (lastItem) {
                if (lastItem.type === 'table') {
                    // Pega a página salva
                    let savedPage = parseInt(localStorage.getItem('sqlite-studio-current-page') || '1');

                    // Verifica se a tabela existe
                    const { db } = await import('./state.js');

                    const tablesResult = await db.query(
                        "SELECT name FROM sqlite_master WHERE type='table'",
                        []
                    );
                    const availableTables = tablesResult.result?.resultRows || [];
                    const tableNames = availableTables.map(t => t.name || t);

                    if (tableNames.includes(lastItem.name)) {
                        setCurrentTable(lastItem.name);

                        // 🔥 CONTA TOTAL DE REGISTROS PRA VALIDAR A PÁGINA
                        const countResult = await db.query(
                            `SELECT COUNT(*) as count FROM ${lastItem.name}`,
                            []
                        );
                        const total = countResult.result?.resultRows[0]?.count || 0;
                        const totalPages = Math.ceil(total / pageSize);

                        // 🔥 VALIDA SE A PÁGINA SALVA É VÁLIDA
                        if (savedPage > totalPages) {
                            savedPage = 1;
                            localStorage.setItem('sqlite-studio-current-page', '1');
                        }

                        await loadTableData(lastItem.name, savedPage);
                        markActiveTreeItem('table', lastItem.name);
                    }
                    else if (tableNames.length > 0) {
                        console.log('📋 Last table not found, loading first available:', tableNames[0]);
                        setCurrentTable(tableNames[0]);
                        await loadTableData(tableNames[0], 1);
                        markActiveTreeItem('table', tableNames[0]);

                        localStorage.setItem('sqlite-studio-last-item', JSON.stringify({
                            type: 'table',
                            name: tableNames[0]
                        }));
                    }

                } else if (lastItem.type === 'view') {
                    // Pega a página salva para views
                    let savedPage = parseInt(localStorage.getItem('sqlite-studio-current-page') || '1');

                    // Verifica se a view existe
                    const { db, pageSize } = await import('./state.js');

                    const viewsResult = await db.query(
                        "SELECT name FROM sqlite_master WHERE type='view'",
                        []
                    );
                    const availableViews = viewsResult.result?.resultRows || [];
                    const viewNames = availableViews.map(v => v.name || v);

                    if (viewNames.includes(lastItem.name)) {
                        console.log('👁 Restoring last view:', lastItem.name);
                        setCurrentView(lastItem.name);

                        try {
                            // 🔥 TENTA CONTAR TOTAL DE REGISTROS PRA VALIDAR A PÁGINA
                            let total = 0;
                            try {
                                const countResult = await db.query(`SELECT COUNT(*) as count FROM ${lastItem.name}`, []);
                                total = countResult.result?.resultRows[0]?.count || 0;
                            } catch (e) {
                                // Se não conseguir contar, usa LIMIT 1 pra estimar
                                const testResult = await db.query(`SELECT * FROM ${lastItem.name} LIMIT 1`, []);
                                total = testResult.result?.resultRows?.length || 1;
                            }

                            const totalPages = Math.ceil(total / pageSize);

                            // 🔥 VALIDA SE A PÁGINA SALVA É VÁLIDA
                            if (savedPage > totalPages) {
                                savedPage = 1;
                                localStorage.setItem('sqlite-studio-current-page', '1');
                            }

                            setCurrentPage(savedPage);
                            const offset = (savedPage - 1) * pageSize;
                            const result = await db.query(`SELECT * FROM ${lastItem.name} LIMIT ${pageSize} OFFSET ${offset}`, []);
                            const rows = result.result?.resultRows || [];

                            if (rows.length === 0) {
                                showNoResults();
                                setTotalRows(0);
                            } else {
                                renderTable(rows);
                                elements.resultsHeader.innerHTML = `<i class="fas fa-eye"></i> View: ${lastItem.name} · ${total} rows`;
                                setTotalRows(total);
                            }

                            updatePagination();
                            markActiveTreeItem('view', lastItem.name);

                        } catch (error) {
                            console.error('Failed to load view:', error);
                            showNoResults();
                            setTotalRows(0);
                            updatePagination();
                        }
                    }
                    else if (viewNames.length > 0) {
                        console.log('👁 Last view not found, loading first available:', viewNames[0]);
                        setCurrentView(viewNames[0]);
                        setCurrentPage(1);

                        try {
                            const result = await db.query(`SELECT * FROM ${viewNames[0]} LIMIT ${pageSize} OFFSET 0`, []);
                            const rows = result.result?.resultRows || [];

                            let total = rows.length;
                            try {
                                const countResult = await db.query(`SELECT COUNT(*) as count FROM ${viewNames[0]}`, []);
                                total = countResult.result?.resultRows[0]?.count || rows.length;
                            } catch (e) { }

                            if (rows.length === 0) {
                                showNoResults();
                                setTotalRows(0);
                            } else {
                                renderTable(rows);
                                elements.resultsHeader.innerHTML = `<i class="fas fa-eye"></i> View: ${viewNames[0]} · ${total} rows`;
                                setTotalRows(total);
                            }

                            updatePagination();
                            markActiveTreeItem('view', viewNames[0]);

                            localStorage.setItem('sqlite-studio-last-item', JSON.stringify({
                                type: 'view',
                                name: viewNames[0]
                            }));
                        } catch (error) {
                            console.error('Failed to load view:', error);
                            showNoResults();
                            setTotalRows(0);
                            updatePagination();
                        }
                    }
                    else {
                        showNoResults();
                        setTotalRows(0);
                        updatePagination();
                    }
                }
            }
        } else {
            // 7⃣ Se não, mostra mensagem de no database
            setCurrentDatabase(null);
            setCurrentTable(null);
            setCurrentView(null);
            setTotalRows(0);
            showNoDatabases();
        }

        toggleDeleteButton(!!lastDb);
        // 8️⃣ Atualiza dropdown
        updateDatabaseSelector();
        await populateDatabaseDropdown();

        // 9️⃣ Atualiza botão Run
        setTimeout(() => {
            updateRunButtonState();
        }, 150);

    } catch (error) {
        console.error('❌ Initialization failed:', error);
        showNoDatabases();
    }
}


// ===== EVENT LISTENERS =====
function setupEventListeners() {

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

    function toggleDeleteButton(show) {
        const deleteBtn = document.getElementById('deleteDbBtn');
        if (deleteBtn) {
            if (show) {
                deleteBtn.classList.remove('hidden');
            } else {
                deleteBtn.classList.add('hidden');
            }
        }
    }

    // 🔥 NOVO: Delete Database button
    const deleteDbBtn = document.getElementById('deleteDbBtn');
    if (deleteDbBtn) {
        deleteDbBtn.addEventListener('click', async () => {
            const { currentDatabase, setCurrentDatabase, setAvailableDatabases } = await import('./state.js');
            const { switchDatabase, updateDatabaseSelector, populateDatabaseDropdown } = await import('./ui.js');
            const { scanOPFSDatabases } = await import('./database.js');  // ← AQUI!

            if (!currentDatabase) {
                alert('No database selected');
                return;
            }

            if (confirm(`Delete database "${currentDatabase}"?`)) {
                try {

                    // Fecha o banco atual primeiro
                    // const { close } = await import('/sqlite-wasm.js');
                    await close();

                    // Deleta o arquivo via OPFS
                    const root = await navigator.storage.getDirectory();
                    await root.removeEntry(currentDatabase);

                    // Atualiza a lista de bancos
                    const databases = await scanOPFSDatabases();
                    setAvailableDatabases(databases);

                    // Limpa o estado do front
                    await switchDatabase(null);
                    toggleDeleteButton(false);
                    await updateDatabaseSelector();
                    await populateDatabaseDropdown();

                } catch (error) {
                    console.error('Failed to delete database:', error);
                    alert('Error deleting database');
                }
            }
        });
    }

    // ===== EDITOR =====
    if (elements.sqlEditor) {

        elements.sqlEditor.addEventListener('input', () => {
            localStorage.setItem('sqlite-studio-last-query', elements.sqlEditor.value);
            updateRunButtonState();
        });

        elements.sqlEditor.addEventListener('keydown', (e) => {
            if (e.ctrlKey && e.key === 'Enter') {
                e.preventDefault();
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
    setupEventListeners();
    initialize();
});
