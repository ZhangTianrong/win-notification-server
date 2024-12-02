# Windows 11 Notification Server

A Rust-based REST API server that enables sending Windows 11 notifications programmatically. Supports both simple text/image notifications and complex XML-based notifications with custom actions and callbacks.

## Features

- REST API for sending notifications
- Support for simple text and image-based notifications
- Support for complex XML-based notifications
- Custom action callbacks
- Command execution support
- Automatic Windows notification registration

## Setup

1. Ensure you have Rust installed on your Windows 11 system
2. Clone this repository
3. Build and run the server:
```bash
cargo build --release
cargo run --release
```

The server will start on `http://localhost:3000`.

## API Endpoints

### POST /notify

Send a notification with the following JSON body:

```json
{
    "title": "Notification Title",
    "message": "Notification Message",
    "image_data": "Optional base64 encoded image",
    "xml_payload": "Optional custom XML payload",
    "callback_command": "Optional command to execute on activation"
}
```

#### Simple Notification Example

```bash
curl -X POST http://localhost:3000/notify \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Hello",
    "message": "This is a test notification"
  }'
```

#### Image Notification Example

```bash
curl -X POST http://localhost:3000/notify \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Image Notification",
    "message": "This notification includes an image",
    "image_data": "<base64-encoded-image-data>"
  }'
```

#### Custom XML Notification Example

```bash
curl -X POST http://localhost:3000/notify \
  -H "Content-Type: application/json" \
  -d '{
    "xml_payload": "<toast><visual><binding template=\"ToastGeneric\"><text>Custom XML Title</text><text>Custom XML message with actions</text></binding></visual><actions><action content=\"Click Me\" arguments=\"custom-action\"/></actions></toast>"
  }'
```

#### Notification with Command Callback

```bash
curl -X POST http://localhost:3000/notify \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Command Notification",
    "message": "Click to execute command",
    "callback_command": "echo Hello from notification!"
  }'
```

## XML Notification Schema

The server supports the full Windows 11 toast notification schema. Some examples of supported elements:

```xml
<toast>
    <visual>
        <binding template="ToastGeneric">
            <text>Title</text>
            <text>Message</text>
            <image placement="appLogoOverride" src="image-url"/>
        </binding>
    </visual>
    <actions>
        <action content="Button Text" arguments="action-argument"/>
    </actions>
    <audio src="ms-winsoundevent:Notification.Default"/>
</toast>
```

For more details on the toast schema, refer to the [Microsoft Toast Content Schema documentation](https://learn.microsoft.com/en-us/windows/apps/design/shell/tiles-and-notifications/toast-schema).

## Error Handling

The server returns appropriate HTTP status codes:

- 200: Notification sent successfully
- 500: Internal server error with error message in response body

## Security Considerations

- The server runs locally and should not be exposed to the public internet
- Callback commands are executed with the same privileges as the server process
- Validate and sanitize all input, especially custom XML payloads and callback commands

## Requirements

- Windows 11
- Rust 1.70 or later
- Administrative privileges (for notification registration)
