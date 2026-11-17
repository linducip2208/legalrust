<# LegalRust autonomous resume controller.
Reads the persistent task queue + NEXT-EXECUTION.md, launches OpenCode
with resume instructions, preserves logs, enforces max retries with
backoff, never runs destructive ops unauthorised. #>
param(
  [string]$Repo = 'D:/project laravel/legalrust',
  [string]$QueueFile = 'docs/AUTONOMOUS/TASK-QUEUE.json',
  [string]$NextFile = 'docs/AUTONOMOUS/NEXT-EXECUTION.md',
  [string]$LogDir = 'logs/autonomous',
  [int]$MaxRetries = 3,
  [int]$BackoffSecs = 120,
  [switch]$Execute
)
$ErrorActionPreference = 'Stop'
Set-Location $Repo
New-Item -ItemType Directory -Force (Join-Path $Repo $LogDir) | Out-Null
function Write-Log($f, $m) {
  $ts = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
  Add-Content (Join-Path $Repo "$LogDir/$f") "[$ts] $m"
}
$queue = Get-Content (Join-Path $Repo $QueueFile) -Raw | ConvertFrom-Json
$next = Get-Content (Join-Path $Repo $NextFile) -Raw
$destructive = @('reset --hard', 'force-push', 'force push', 'DROP DATABASE', 'drop table', 'rm -rf')
foreach ($line in ($next -split "`n")) {
  foreach ($pat in $destructive) {
    if ($line.ToLower().Contains($pat.ToLower())) {
      Write-Log 'errors.log' "REFUSED destructive instruction in NEXT-EXECUTION.md: $line"
      Write-Output 'REFUSED: destructive instruction requires explicit approval.'
      exit 2
    }
  }
}
$todo = @($queue.tasks | Where-Object { $_.status -eq 'TODO' -or $_.status -eq 'IN_PROGRESS' })
if (-not $todo -or $todo.Count -eq 0) {
  Write-Log 'execution.log' 'Queue empty: nothing to resume. Re-scan required before claiming completion.'
  Write-Output 'QUEUE EMPTY - re-scan feature catalog/routes before completion claim.'
  exit 0
}
$open = $todo | Select-Object -First 1
Write-Log 'execution.log' ("NEXT TASK: " + $open.id + " - " + $open.title + " (status=" + $open.status + ")")
Write-Log 'session-history.log' ("Starting task " + $open.id)
$resume = "AUTONOMOUS RESUME - LegalRust. Repo: $Repo. Read docs/AUTONOMOUS/MASTER-PROGRESS.md, TASK-QUEUE.json, NEXT-EXECUTION.md first. Current task: $($open.id) - $($open.title). Rules: AGENTS.md plus non-negotiable rules (no fake data, no destructive migrations, no force-push, no secrets). Verify with cargo fmt/check/clippy/test. Update TASK-QUEUE.json status plus NEXT-EXECUTION.md when done. Continue to the next TODO task after this one if unblocked; stop and record blockers otherwise."
$attempt = 0
$ok = $false
while (($attempt -lt $MaxRetries) -and (-not $ok)) {
  $attempt = $attempt + 1
  Write-Log 'execution.log' ("Attempt " + $attempt + "/" + $MaxRetries + " for " + $open.id)
  try {
    if ($Execute) {
      $out = & opencode run "$resume" 2>&1 | Out-String
      Add-Content (Join-Path $Repo "$LogDir/agent-results.log") $out
      if ($LASTEXITCODE -eq 0) { $ok = $true }
      else { Write-Log 'errors.log' ("attempt " + $attempt + " exit=" + $LASTEXITCODE) }
    } else {
      Write-Output '--- RESUME PROMPT (dry-run; pass -Execute to run) ---'
      Write-Output $resume
      $ok = $true
    }
  } catch {
    Write-Log 'errors.log' ("attempt " + $attempt + " exception: " + $_.Exception.Message)
  }
  if ((-not $ok) -and ($attempt -lt $MaxRetries)) {
    $wait = $BackoffSecs * [math]::Pow(2, ($attempt - 1))
    Write-Log 'execution.log' ("Backing off " + $wait + "s before retry")
    Start-Sleep -Seconds $wait
  }
}
if (-not $ok) {
  Write-Log 'errors.log' ("Task " + $open.id + " failed after " + $MaxRetries + " attempts - recorded, NOT retried further.")
  Write-Output "FAILED after $MaxRetries attempts - see logs/autonomous/errors.log. Will not loop."
  exit 1
}
Write-Log 'execution.log' ("Task " + $open.id + " controller finished. Verify TASK-QUEUE.json updated by agent.")
Write-Output 'DONE - verify docs/AUTONOMOUS/TASK-QUEUE.json and git status before next wave.'
