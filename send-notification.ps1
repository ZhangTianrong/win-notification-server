param(
    [Parameter(Mandatory=$true)]
    [string]$title,
    
    [Parameter(Mandatory=$true)]
    [string]$message,

    [Parameter(Mandatory=$false)]
    [string]$imagePath
)

# Create multipart/form-data content
$boundary = [System.Guid]::NewGuid().ToString()
$LF = "`r`n"

# Create a memory stream to write the multipart form data
$stream = New-Object System.IO.MemoryStream

# Helper function to write string to stream
function Write-ToStream {
    param($stream, $string)
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($string)
    $stream.Write($bytes, 0, $bytes.Length)
}

# Write title field
Write-ToStream $stream "--$boundary$LF"
Write-ToStream $stream "Content-Disposition: form-data; name=`"title`"$LF"
Write-ToStream $stream "Content-Type: text/plain; charset=utf-8$LF$LF"
Write-ToStream $stream "$title$LF"

# Write message field
Write-ToStream $stream "--$boundary$LF"
Write-ToStream $stream "Content-Disposition: form-data; name=`"message`"$LF"
Write-ToStream $stream "Content-Type: text/plain; charset=utf-8$LF$LF"
Write-ToStream $stream "$message$LF"

# Add image if provided
if ($imagePath -and (Test-Path $imagePath)) {
    $fileName = Split-Path $imagePath -Leaf
    $contentType = switch ([System.IO.Path]::GetExtension($imagePath).ToLower()) {
        ".jpg"  { "image/jpeg" }
        ".jpeg" { "image/jpeg" }
        ".png"  { "image/png" }
        ".gif"  { "image/gif" }
        default { "application/octet-stream" }
    }
    
    Write-ToStream $stream "--$boundary$LF"
    Write-ToStream $stream "Content-Disposition: form-data; name=`"image`"; filename=`"$fileName`"$LF"
    Write-ToStream $stream "Content-Type: $contentType$LF$LF"
    
    # Write image binary data
    $imageBytes = [System.IO.File]::ReadAllBytes($imagePath)
    $stream.Write($imageBytes, 0, $imageBytes.Length)
    Write-ToStream $stream "$LF"
}

# Write final boundary
Write-ToStream $stream "--$boundary--$LF"

# Get the complete body as bytes
$bodyBytes = $stream.ToArray()
$stream.Close()

# Send the request
try {
    $headers = @{
        "Content-Type" = "multipart/form-data; boundary=$boundary"
    }
    
    $response = Invoke-WebRequest -Uri "http://localhost:3000/notify" `
        -Method Post `
        -Headers $headers `
        -Body $bodyBytes `
        -ContentType "multipart/form-data; boundary=$boundary"
    
    # Decode and display the response content
    $responseText = [System.Text.Encoding]::UTF8.GetString($response.Content)
    Write-Host $responseText
}
catch {
    Write-Error "Failed to send notification: $_"
    if ($_.Exception.Response) {
        $reader = [System.IO.StreamReader]::new($_.Exception.Response.GetResponseStream())
        $error_content = $reader.ReadToEnd()
        Write-Error $error_content
        $reader.Close()
    }
}
