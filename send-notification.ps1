param(
    [Parameter(Mandatory=$true)]
    [string]$title,
    
    [Parameter(Mandatory=$true)]
    [string]$message
)

# Convert string to JSON-escaped format with Unicode escapes
function ConvertTo-JsonUnicode {
    param([string]$text)
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($text)
    $escaped = [System.Text.RegularExpressions.Regex]::Replace(
        [System.Text.Encoding]::UTF8.GetString($bytes),
        '[^\x00-\x7F]',
        {
            param($match)
            [string]::Format('\u{0:x4}', [int][char]$match.Value)
        }
    )
    return $escaped
}

$escapedTitle = ConvertTo-JsonUnicode -text $title
$escapedMessage = ConvertTo-JsonUnicode -text $message

$json = "{`"title`":`"$escapedTitle`",`"message`":`"$escapedMessage`"}"

# Send the notification
$response = curl -X POST -H "Content-Type: application/json" -d $json "http://localhost:3000/notify"
Write-Output $response
