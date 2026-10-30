# Save this script as UTF-8 with BOM when invoking Windows PowerShell 5.1.
param([Parameter(Mandatory=$true)][string]$Output)
$ErrorActionPreference = 'Stop'
if (Test-Path -LiteralPath $Output) { throw 'Output exists; choose a fresh WAV path' }
$voice = New-Object -ComObject SAPI.SpVoice
$voices = @($voice.GetVoices())
$voices | ForEach-Object { Write-Host $_.GetDescription() }
$russian = @($voices | Where-Object { $_.GetDescription() -eq 'Microsoft Irina Desktop - Russian' })
if ($russian.Count -ne 1) { throw 'Required Russian voice unavailable; acceptance is incomplete' }
$voice.Voice = $russian[0]
$stream = New-Object -ComObject SAPI.SpFileStream
# SAPI SpeechAudioFormatType SAFT16kHz16BitMono; no speaker output.
$stream.Format.Type = 18
$stream.Open([IO.Path]::GetFullPath($Output), 3, $false)
try {
    $voice.AudioOutputStream = $stream
    $voice.Speak('Сегодня мы проверяем распознавание русской речи. Все четыре модели работают локально на компьютере.') | Out-Null
} finally {
    $stream.Close()
}
