// Test results tracking
let results = {
    total: 41,
    passed: 0,
    failed: 0,
    modules: {
        blobs: { total: 6, passed: 0, failed: 0 },
        bindings: { total: 6, passed: 0, failed: 0 },
        worker: { total: 9, passed: 0, failed: 0 },
        database: { total: 15, passed: 0, failed: 0 },
        batch: { total: 6, passed: 0, failed: 0 },
        pool: { total: 7, passed: 0, failed: 0 },
        prepared: { total: 6, passed: 0, failed: 0 },
        queries: { total: 5, passed: 0, failed: 0 }
    }
};

let startTime = Date.now();
let timerInterval;

// DOM elements
const output = document.getElementById('output');
const progress = document.getElementById('progress');
const statsText = document.getElementById('stats-text');
const statusBadge = document.getElementById('status-badge');
const timerEl = document.getElementById('timer');
const passedCount = document.getElementById('passed-count');
const failedCount = document.getElementById('failed-count');

// Initialize
function init() {
    // Start timer
    timerInterval = setInterval(updateTimer, 1000);
    
    // Override console methods
    const originalLog = console.log;
    const originalError = console.error;
    const originalWarn = console.warn;
    
    console.log = function(...args) {
        processOutput('log', args.join(' '));
        originalLog.apply(console, args);
    };
    
    console.error = function(...args) {
        processOutput('error', args.join(' '));
        originalError.apply(console, args);
    };
    
    console.warn = function(...args) {
        processOutput('warn', args.join(' '));
        originalWarn.apply(console, args);
    };
}

// Process test output
function processOutput(type, message) {
    const line = document.createElement('div');
    
    // Parse test results
    if (message.includes('test ') && message.includes(' ... ok')) {
        line.className = 'ok';
        line.innerHTML = `<i class="fas fa-check-circle"></i> ${message}`;
        updateTestResult('pass', extractModule(message));
    } 
    else if (message.includes('test ') && message.includes(' ... FAIL')) {
        line.className = 'fail';
        line.innerHTML = `<i class="fas fa-times-circle"></i> ${message}`;
        updateTestResult('fail', extractModule(message));
    }
    else if (message.includes('running ')) {
        line.className = 'info';
        line.innerHTML = `<i class="fas fa-play-circle"></i> ${message}`;
    }
    else if (type === 'error') {
        line.className = 'fail';
        line.innerHTML = `<i class="fas fa-exclamation-triangle"></i> ${message}`;
    }
    else if (type === 'warn') {
        line.className = 'pending';
        line.innerHTML = `<i class="fas fa-exclamation-circle"></i> ${message}`;
    }
    else {
        line.className = 'info';
        line.innerHTML = `<i class="fas fa-info-circle"></i> ${message}`;
    }
    
    output.appendChild(line);
    output.scrollTop = output.scrollHeight;
}

// Extract module name from test message
function extractModule(message) {
    const match = message.match(/test (\w+)::/);
    return match ? match[1] : null;
}

// Update test results
function updateTestResult(type, module) {
    if (type === 'pass') {
        results.passed++;
        if (module && results.modules[module]) {
            results.modules[module].passed++;
        }
    } else if (type === 'fail') {
        results.failed++;
        if (module && results.modules[module]) {
            results.modules[module].failed++;
        }
    }
    
    updateUI();
    
    // Check if all tests completed
    if (results.passed + results.failed === results.total) {
        complete();
    }
}

// Update UI
function updateUI() {
    // Progress bar
    const percent = (results.passed / results.total * 100) || 0;
    progress.style.width = percent + '%';
    progress.textContent = percent.toFixed(1) + '%';
    
    // Stats
    statsText.textContent = `${results.passed}/${results.total} passando`;
    passedCount.textContent = results.passed;
    failedCount.textContent = results.failed;
    
    // Update module cards
    for (const [module, data] of Object.entries(results.modules)) {
        const passEl = document.getElementById(`pass-${module}`);
        const statusEl = document.getElementById(`status-${module}`);
        
        if (passEl) {
            passEl.textContent = data.passed;
        }
        
        if (statusEl) {
            if (data.failed > 0) {
                statusEl.className = 'card-status fail';
                statusEl.innerHTML = '<i class="fas fa-times-circle"></i>';
            } else if (data.passed === data.total) {
                statusEl.className = 'card-status pass';
                statusEl.innerHTML = '<i class="fas fa-check-circle"></i>';
            } else if (data.passed > 0 || data.failed > 0) {
                statusEl.className = 'card-status pending';
                statusEl.innerHTML = '<i class="fas fa-spinner fa-spin"></i>';
            }
        }
    }
}

// Update timer
function updateTimer() {
    const elapsed = Math.floor((Date.now() - startTime) / 1000);
    timerEl.textContent = elapsed + 's';
}

// Tests complete
function complete() {
    clearInterval(timerInterval);
    
    if (results.failed === 0) {
        statusBadge.innerHTML = '✅ SUCESSO';
        statusBadge.style.background = '#4caf50';
    } else {
        statusBadge.innerHTML = '❌ FALHAS';
        statusBadge.style.background = '#f44336';
    }
    
    // Add completion message
    const line = document.createElement('div');
    line.className = results.failed === 0 ? 'ok' : 'fail';
    line.innerHTML = `<i class="fas fa-flag-checkered"></i> Testes finalizados! ${results.passed}/${results.total} passaram (${results.failed} falhas)`;
    output.appendChild(line);
}

// Start when page loads
document.addEventListener('DOMContentLoaded', init);
