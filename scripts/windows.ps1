param([ValidateSet('check','test','dev','build')][string]$Action='check')
$ErrorActionPreference='Stop'
$taskRoot=Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $taskRoot
if(Test-Path -LiteralPath (Join-Path $taskRoot '.tools/cargo/bin/cargo.exe')){
  $env:CARGO_HOME=Join-Path $taskRoot '.tools/cargo'
  $env:RUSTUP_HOME=Join-Path $taskRoot '.tools/rustup'
  $env:PATH=(Join-Path $env:CARGO_HOME 'bin')+';'+$env:PATH
}
function Invoke-Checked([scriptblock]$Command){ & $Command; if($LASTEXITCODE -ne 0){throw "Command failed with exit code $LASTEXITCODE"} }
switch($Action){
  'check'{ Invoke-Checked {npm.cmd run check}; Invoke-Checked {cargo check --manifest-path src-tauri/Cargo.toml} }
  'test'{ Invoke-Checked {npm.cmd test}; Invoke-Checked {cargo test --manifest-path src-tauri/Cargo.toml --lib} }
  'dev'{ Invoke-Checked {npm.cmd run app:dev} }
  'build'{ Invoke-Checked {npm.cmd run app:build -- --bundles nsis} }
}
