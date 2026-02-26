const express = require("express");
const path = require("path");

const app = express();
const PORT = 8080;

// 🔥 COOP/COEP PRIMEIRO
app.use((req, res, next) => {
  res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
  res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
  next();
});

// Depois serve arquivos estáticos
app.use(express.static(__dirname, {
  setHeaders: (res, filePath) => {
    if (filePath.endsWith(".wasm")) {
      res.setHeader("Content-Type", "application/wasm");
    }
  }
}));

app.listen(PORT, () =>
  console.log(`Servidor rodando em http://localhost:${PORT}`)
);
