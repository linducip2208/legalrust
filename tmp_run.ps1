$env:RUST_BACKTRACE = '1'
$env:RUST_LOG = 'info'
$env:JWT_SECRET = 'dev-local-secret-0123456789abcdef01234567'
$env:INTEGRATION_SECRET = 'dev-local-int-secret'
$env:DATABASE_URL = 'mysql://root@127.0.0.1:3306/legalrust'
$env:Path += ';C:\Program Files\Tesseract-OCR'
$env:TESSDATA_PREFIX = "$env:LOCALAPPDATA\tessdata"
Set-Location 'D:\project laravel\legalrust'
& '.\target-ui5\debug\legalrust-server.exe' *> 'D:\project laravel\legalrust\tmp_boot.log'
"exit=$LASTEXITCODE" | Out-File 'D:\project laravel\legalrust\tmp_boot.log' -Append
