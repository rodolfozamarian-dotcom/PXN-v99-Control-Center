# Simula o mod FS25_PXNV99Telemetry: conecta no pipe do app e envia quadros
# de um trator acelerando em lavoura. Serve para testar o app sem o jogo.
#
# Uso: powershell -ExecutionPolicy Bypass -File tools/fake-fs25-telemetry.ps1 [-Seconds 5]
param([int]$Seconds = 5, [int]$Hz = 100)

$pipe = New-Object System.IO.Pipes.NamedPipeClientStream('.', 'PXNV99Telemetry', [System.IO.Pipes.PipeDirection]::Out)
$pipe.Connect(5000)
$w = New-Object System.IO.StreamWriter($pipe)
$w.AutoFlush = $true
$inv = [Globalization.CultureInfo]::InvariantCulture

$frames = $Seconds * $Hz
for ($i = 0; $i -lt $frames; $i++) {
    $p = $i / [double]$frames
    $rpm = 850 + 1350 * $p
    $line = [string]::Format($inv,
        '{{"v":1,"t":{0},"pz":false,"in":true,"on":true,"rpm":{1:F1},"rmin":850,"rmax":2200,"ld":{2:F3},"thr":{3:F3},"clt":{4:F3},"gr":3,"spd":{5:F2},"wn":4,"wc":4,"gRd":0,"gHd":0,"gFd":4,"gSf":0,"dep":0.031,"slp":0.08}}',
        [int]($i * 1000 / $Hz), $rpm, (0.9 - 0.6 * $p), $p, [math]::Max(0, 1 - 4 * $p), (2 + 18 * $p))
    $w.WriteLine($line)
    if ($i -eq 10) { $w.WriteLine('isto nao e json') }
    Start-Sleep -Milliseconds ([int](1000 / $Hz))
}
$w.WriteLine('{"v":1,"msg":"teste de mensagem do mod"}')
$w.Dispose()
"enviados $frames quadros"
