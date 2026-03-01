//webserver.js
const express = require("express");
const compression = require("compression");
const path = require("path");

const app = express();
const PORT = 8080;

// Enable compression for all responses
app.use(compression({
  level: 9,              // Maximum compression
  threshold: 512,        // Compress files > 512 bytes
  filter: (req, res) => {
    // Skip already compressed formats
    if (req.path.match(/\.(jpg|jpeg|png|gif|mp4)$/)) {
      return false;
    }
    return compression.filter(req, res);
  }
}));

// COOP/COEP headers (required for OPFS)
app.use((_req, res, next) => {
  res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
  res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
  res.setHeader("Cache-Control", "no-cache");
  next();
});

// Serve static files
app.use(express.static(__dirname));
app.use("/pkg", express.static(path.join(__dirname, "pkg")));
app.use("/sqlite-wasm", express.static(path.join(__dirname, "sqlite-wasm")));

app.listen(PORT, () =>
  console.log(`🚀 Server running at http://localhost:${PORT} (with compression)`)
);
