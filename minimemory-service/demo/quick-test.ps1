# minimemory-service Quick Test (PowerShell)
# Run the server first: npm run dev

$API = "http://localhost:3000/api/v1"
$Headers = @{
    "X-API-Key" = "mm_dev_key_12345"
    "Content-Type" = "application/json"
}

Write-Host "======================================" -ForegroundColor Cyan
Write-Host "  minimemory-service Quick Test" -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

# 1. Health check
Write-Host "1. Health Check" -ForegroundColor Yellow
(Invoke-RestMethod -Uri "http://localhost:3000/health") | ConvertTo-Json
Write-Host ""

# 2. Remember a memory
Write-Host "2. Remember - Store a memory" -ForegroundColor Yellow
$body = @{
    content = "User prefers dark mode in the application"
    embedding = @(0.9, 0.1, 0.2, 0.1, 0.0)
    type = "semantic"
    importance = 0.8
    metadata = @{
        category = "preferences"
        userId = "user-123"
    }
} | ConvertTo-Json -Depth 3

$result = Invoke-RestMethod -Uri "$API/remember" -Method POST -Headers $Headers -Body $body
$result | ConvertTo-Json -Depth 3
$memoryId = $result.memory.id
Write-Host "Memory ID: $memoryId" -ForegroundColor Green
Write-Host ""

# 3. Add more memories
Write-Host "3. Adding more memories..." -ForegroundColor Yellow
@(
    @{content="User asked about enterprise pricing"; embedding=@(0.85,0.15,0.25,0.1,0.0); type="episodic"; importance=0.7},
    @{content="User works at TechCorp Inc"; embedding=@(0.8,0.2,0.3,0.15,0.0); type="semantic"; importance=0.9},
    @{content="User requested API documentation"; embedding=@(0.1,0.9,0.1,0.8,0.7); type="episodic"; importance=0.6}
) | ForEach-Object {
    $body = $_ | ConvertTo-Json
    Invoke-RestMethod -Uri "$API/remember" -Method POST -Headers $Headers -Body $body | Out-Null
    Write-Host "  Added: $($_.content.Substring(0, [Math]::Min(40, $_.content.Length)))..." -ForegroundColor Gray
}
Write-Host ""

# 4. Stats
Write-Host "4. Get Stats" -ForegroundColor Yellow
(Invoke-RestMethod -Uri "$API/stats" -Headers $Headers) | ConvertTo-Json -Depth 3
Write-Host ""

# 5. Vector recall
Write-Host "5. Recall - Vector Search" -ForegroundColor Yellow
$body = @{
    embedding = @(0.88, 0.12, 0.22, 0.12, 0.0)
    mode = "vector"
    limit = 3
} | ConvertTo-Json

$results = Invoke-RestMethod -Uri "$API/recall" -Method POST -Headers $Headers -Body $body
Write-Host "Found $($results.count) memories:" -ForegroundColor Green
$results.results | ForEach-Object {
    Write-Host "  - [$($_.type)] $($_.content) (score: $([math]::Round($_.score, 3)))" -ForegroundColor White
}
Write-Host ""

# 6. Keyword recall
Write-Host "6. Recall - Keyword Search" -ForegroundColor Yellow
$body = @{
    keywords = "user preferences dark mode"
    mode = "keyword"
    limit = 3
} | ConvertTo-Json

$results = Invoke-RestMethod -Uri "$API/recall" -Method POST -Headers $Headers -Body $body
Write-Host "Found $($results.count) memories:" -ForegroundColor Green
$results.results | ForEach-Object {
    Write-Host "  - [$($_.type)] $($_.content) (score: $([math]::Round($_.score, 3)))" -ForegroundColor White
}
Write-Host ""

# 7. Get specific memory
Write-Host "7. Get Memory by ID" -ForegroundColor Yellow
$memory = Invoke-RestMethod -Uri "$API/memory/$memoryId" -Headers $Headers
$memory.memory | ConvertTo-Json -Depth 2
Write-Host ""

# 8. Export
Write-Host "8. Export All" -ForegroundColor Yellow
$export = Invoke-RestMethod -Uri "$API/export" -Method POST -Headers $Headers
Write-Host "Exported $($export.data.memories.Count) memories" -ForegroundColor Green
Write-Host ""

# 9. Clear (optional)
Write-Host "9. Clear All Memories" -ForegroundColor Yellow
$clear = Invoke-RestMethod -Uri "$API/clear" -Method DELETE -Headers $Headers
$clear | ConvertTo-Json
Write-Host ""

Write-Host "======================================" -ForegroundColor Cyan
Write-Host "  Test Complete!" -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
