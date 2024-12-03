curl.exe -X POST `
  -F "title=测试通知 / テスト通知" `
  -F "message=点击复制文本，关闭无动作 / クリックでコピー、閉じても何もしない" `
  -F "image=@test-image.jpg" `
  -F "files=@test-attachment.txt" `
  http://localhost:3000/notify

# Example with multiple files:
# curl.exe -X POST `
#   -F "title=测试通知 / テスト通知" `
#   -F "message=点击复制文本，关闭无动作 / クリックでコピー、閉じても何もしない" `
#   -F "image=@test-image.jpg" `
#   -F "files=@test-attachment.txt" `
#   -F "files=@test-attachment2.txt" `
#   http://localhost:3000/notify

# Example with files only (no image):
# curl.exe -X POST `
#   -F "title=测试通知 / テスト通知" `
#   -F "message=点击复制文本，关闭无动作 / クリックでコピー、閉じても何もしない" `
#   -F "files=@test-attachment.txt" `
#   http://localhost:3000/notify
