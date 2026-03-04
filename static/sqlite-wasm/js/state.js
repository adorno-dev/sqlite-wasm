export let db = null;
export let currentDatabase = null;
export let availableDatabases = [];
export let currentTable = null;
export let currentView = null;
export let currentPage = 1;
export let pageSize = 25;  // ← NÚMERO
export let totalRows = 0;
export let lastQuery = null;
export let lastResults = [];

export const elements = {
    dbSelector: document.getElementById('selectedDb'),
    dbDropdown: document.getElementById('dbDropdown'),
    dbSelectorSpan: document.querySelector('#selectedDb span'),
    
    treeTables: document.getElementById('treeTables'),
    treeViews: document.getElementById('treeViews'),
    treeTriggers: document.getElementById('treeTriggers'),
    
    sqlEditor: document.getElementById('sqlEditor'),
    runBtn: document.getElementById('runBtn'),
    formatBtn: document.getElementById('formatBtn'),
    clearBtn: document.getElementById('clearBtn'),
    refreshBtn: document.getElementById('refreshBtn'),
    
    resultsHeader: document.querySelector('.results-header span'),
    tableWrapper: document.getElementById('tableWrapper'),
    
    paginationInfo: document.querySelector('.pagination-info span'),
    prevPageBtn: document.getElementById('prevPage'),
    nextPageBtn: document.getElementById('nextPage'),
    pageSizeSelect: document.getElementById('pageSize'),  // ← ELEMENTO DOM
    pageNumbers: document.getElementById('pageNumbers'),
    
    themeToggle: document.getElementById('themeToggle'),
    
    exportCsvBtn: document.querySelector('[title="Export CSV"]'),
    exportJsonBtn: document.querySelector('[title="Copy as JSON"]')
};

export function setDb(newDb) { 
    db = newDb; 
}
export function setCurrentDatabase(newDb) { 
    currentDatabase = newDb; 
    if (newDb) {
        localStorage.setItem('sqlite-studio-last-db', newDb);
    } else {
        localStorage.removeItem('sqlite-studio-last-db');
    }
}
export function setCurrentTable(newTable) { 
    currentTable = newTable; 
    currentView = null;
    if (newTable) {
        localStorage.setItem('sqlite-studio-last-item', JSON.stringify({
            type: 'table',
            name: newTable
        }));
    }
}

export function setCurrentView(newView) { 
    currentView = newView; 
    currentTable = null;
    if (newView) {
        localStorage.setItem('sqlite-studio-last-item', JSON.stringify({
            type: 'view',
            name: newView
        }));
    }
}
export function setCurrentPage(newPage) { 
    currentPage = newPage;
    // Salva a página atual no storage se tiver uma tabela ativa
    if (currentTable) {
        localStorage.setItem('sqlite-studio-current-page', newPage.toString());
    }
}
export function setAvailableDatabases(newDbs) { availableDatabases = newDbs; }
export function setPageSize(newSize) { pageSize = newSize; }
export function setTotalRows(newTotal) { totalRows = newTotal; }
export function setLastQuery(newQuery) { lastQuery = newQuery; }
export function setLastResults(newResults) { lastResults = newResults; }
