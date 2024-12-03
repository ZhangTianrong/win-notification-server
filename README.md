# Windows 11 Notification Server

A Rust-based REST API server that enables sending Windows 11 notifications programmatically. Supports text notifications with images, file attachments, and callback actions.

## Features

- REST API for sending notifications
- Support for text notifications with images
- Support for file attachments
- Custom action callbacks
- Command execution support
- Automatic Windows notification registration
- Configurable port and bind address
- Basic authentication for non-localhost requests
- Default action of copying the text to clipboard and revealing the attachments in explorer

## Setup

1. Ensure you have Rust installed on your Windows 11 system
2. Clone this repository
3. Build and run the server:
```bash
cargo build --release
cargo run --release
```

By default, the server will start on `http://localhost:3000`.

### Command Line Arguments

The server supports the following command line arguments:

```bash
USAGE:
    notification_server [OPTIONS]

Options:
    -a, --address <ADDRESS>     Address to listen on [default: 0.0.0.0]
    -p, --port <PORT>           Port to listen on [default: 3000]
    -u, --username <USERNAME>   Optional username for basic authentication
    -w, --password <PASSWORD>   Optional password for basic authentication
    -h, --help                  Print help
    -V, --version               Print version
```

Example with custom port and authentication:
```bash
cargo run --release -- --port 8080 --bind 0.0.0.0 --username admin --password secret
```

## API Endpoints

### POST /notify

Send a notification using multipart form data with the following fields:

- `title`: The notification title (required)
- `message`: The notification message (required)
- `image`: An image file to display in the notification (optional)
- `image_position`: Wether to display the image as a banner or a logo (optional)
- `files`: One or more file attachments (optional, can be specified multiple times)
- `callback_command`: Command to execute when the notification is clicked (optional)

#### Basic Notification Example (localhost)

```bash
curl -X POST http://localhost:3000/notify \
  -F "title=Hello" \
  -F "message=This is a test notification"
```

#### Example with Authentication (non-localhost)

```bash
curl -X POST http://example.com:3000/notify \
  -u "username:password" \
  -F "title=Hello" \
  -F "message=This is a test notification"
```

#### Image Notification Example

```bash
curl -X POST http://localhost:3000/notify \
  -F "title=Image Notification" \
  -F "message=This notification includes an image" \
  -F "image=@/path/to/image.jpg"
```

#### Notification with File Attachments

```bash
curl -X POST http://localhost:3000/notify \
  -F "title=File Notification" \
  -F "message=This notification includes files" \
  -F "files=@/path/to/file1.txt" \
  -F "files=@/path/to/file2.txt"
```

#### Notification with Command Callback

```bash
curl -X POST http://localhost:3000/notify \
  -F "title=Command Notification" \
  -F "message=Click to execute command" \
  -F "callback_command=start https://example.com"
```

## Error Handling

The server returns appropriate HTTP status codes:

- 200: Notification sent successfully
- 401: Unauthorized (invalid or missing authentication credentials)
- 500: Internal server error with error message in response body

## Security Considerations

- The server should be configured appropriately when exposed to non-localhost requests
- Use strong authentication credentials when enabling non-localhost access
- Callback commands are executed with the same privileges as the server process
- Validate and sanitize all input, especially callback commands
- Consider using HTTPS in production environments when accepting non-localhost requests

## Requirements

- Windows 11
- Rust 1.70 or later
- Administrative privileges (for notification registration)
