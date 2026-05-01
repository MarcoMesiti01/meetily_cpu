#!/usr/bin/env node
/**
 * Auto-detect GPU and run Tauri with appropriate features
 */

const { execSync } = require('child_process');
const path = require('path');
const fs = require('fs');
const os = require('os');

// Get the command (dev or build)
const command = process.argv[2];
if (!command || !['dev', 'build'].includes(command)) {
  console.error('Usage: node tauri-auto.js [dev|build]');
  process.exit(1);
}

// Detect GPU feature
let feature = '';

// Check for environment variable override first
if (process.env.TAURI_GPU_FEATURE) {
  feature = process.env.TAURI_GPU_FEATURE;
  console.log(`🔧 Using forced GPU feature from environment: ${feature}`);
} else {
  try {
    const result = execSync('node scripts/auto-detect-gpu.js', {
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'inherit']
    });
    feature = result.trim();
  } catch (err) {
    // If detection fails, continue with no features
  }
}

console.log(''); // Empty line for spacing

// Platform-specific environment variables
const platform = os.platform();
const env = { ...process.env };

function prependToPath(dir) {
  const pathKey = Object.keys(env).find((key) => key.toLowerCase() === 'path') || 'PATH';
  env[pathKey] = `${dir}${path.delimiter}${env[pathKey] || ''}`;
}

function configureLibclangOnWindows() {
  if (platform !== 'win32' || env.LIBCLANG_PATH) {
    return;
  }

  const candidates = [
    'C:\\Program Files\\LLVM\\bin',
    'C:\\Program Files (x86)\\LLVM\\bin',
    'C:\\ProgramData\\chocolatey\\lib\\llvm\\tools\\LLVM\\bin',
  ];

  for (const candidate of candidates) {
    if (fs.existsSync(path.join(candidate, 'libclang.dll'))) {
      env.LIBCLANG_PATH = candidate;
      prependToPath(candidate);
      console.log(`🔧 Using libclang from: ${candidate}`);
      return;
    }
  }

  console.warn('⚠️  libclang.dll was not found in common LLVM locations.');
  console.warn('   If the Rust build fails in whisper-rs-sys, install LLVM and retry:');
  console.warn('   winget install LLVM.LLVM   (or: choco install llvm -y)');
}

function configureCmakeOnWindows() {
  if (platform !== 'win32') {
    return;
  }

  const candidates = [
    'C:\\Program Files\\CMake\\bin',
    'C:\\Program Files (x86)\\CMake\\bin',
    'C:\\ProgramData\\chocolatey\\bin',
  ];

  for (const candidate of candidates) {
    if (fs.existsSync(path.join(candidate, 'cmake.exe'))) {
      prependToPath(candidate);
      console.log(`🔧 Using CMake from: ${candidate}`);
      return;
    }
  }

  console.warn('⚠️  cmake.exe was not found in common locations.');
  console.warn('   If the Rust build fails in whisper-rs-sys, install CMake and retry:');
  console.warn('   winget install Kitware.CMake   (or: choco install cmake -y)');
}

configureLibclangOnWindows();
configureCmakeOnWindows();

function ensureLlamaSidecarForDev() {
  if (command !== 'dev') return;

  const workspaceRoot = path.resolve(process.cwd(), '..');
  const sidecarDir = path.resolve(process.cwd(), 'src-tauri', 'binaries');
  const sidecarPath = path.join(sidecarDir, 'llama-helper-x86_64-pc-windows-msvc.exe');

  if (platform !== 'win32') return;
  if (fs.existsSync(sidecarPath)) return;

  console.log('🔧 Building llama-helper sidecar for local dev...');
  if (!fs.existsSync(sidecarDir)) {
    fs.mkdirSync(sidecarDir, { recursive: true });
  }

  execSync('cargo build --release -p llama-helper', {
    stdio: 'inherit',
    cwd: workspaceRoot,
    env,
  });

  const builtSidecar = path.join(workspaceRoot, 'target', 'release', 'llama-helper.exe');
  if (!fs.existsSync(builtSidecar)) {
    throw new Error(`Expected sidecar was not built: ${builtSidecar}`);
  }

  fs.copyFileSync(builtSidecar, sidecarPath);
  console.log(`✅ Sidecar ready: ${sidecarPath}`);
}

ensureLlamaSidecarForDev();

function ensureWindowsRuntimeForBuild() {
  if (command !== 'build' || platform !== 'win32') return;
  if (env.MEETILY_SKIP_RUNTIME_PACKAGE === '1') {
    console.log('Skipping bundled runtime packaging because MEETILY_SKIP_RUNTIME_PACKAGE=1');
    return;
  }

  console.log('Preparing bundled Windows Python runtime...');
  execSync('powershell -ExecutionPolicy Bypass -File scripts/package-windows-runtime.ps1', {
    stdio: 'inherit',
    cwd: process.cwd(),
    env,
  });
  execSync('powershell -ExecutionPolicy Bypass -File scripts/smoke-bundled-runtime.ps1 -RequireModel', {
    stdio: 'inherit',
    cwd: process.cwd(),
    env,
  });
}

ensureWindowsRuntimeForBuild();

if (platform === 'linux' && feature === 'cuda') {
  console.log('🐧 Linux/CUDA detected: Setting CMAKE flags for NVIDIA GPU');
  env.CMAKE_CUDA_ARCHITECTURES = '75';
  env.CMAKE_CUDA_STANDARD = '17';
  env.CMAKE_POSITION_INDEPENDENT_CODE = 'ON';
}

// Build the tauri command
let tauriCmd = `tauri ${command}`;
if (feature && feature !== 'none') {
  tauriCmd += ` -- --features ${feature}`;
  console.log(`🚀 Running: tauri ${command} with features: ${feature}`);
} else {
  console.log(`🚀 Running: tauri ${command} (CPU-only mode)`);
}
console.log('');

// Execute the command
try {
  execSync(tauriCmd, { stdio: 'inherit', env });
} catch (err) {
  process.exit(err.status || 1);
}
