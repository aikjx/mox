fpath = r'D:\a10\aikjx\gitcode\infotopograph\platform\foundation\mox-api-protocol\src\lib.rs'
with open(fpath, encoding='utf-8', errors='replace') as f:
    c = f.read()

# 1. 结构体字段
c = c.replace('    pub message: String,', '    pub msg: String,')

# 2. 构造函数中的 message: → msg: (注意要精确匹配，避免误改参数名)
c = c.replace('            message: "ok".into(),', '            msg: "ok".into(),')
c = c.replace('            message: message.into(),', '            msg: message.into(),')
c = c.replace('            message: err.message.clone(),', '            msg: err.message.clone(),')

# 3. map 函数中的 self.message → self.msg
c = c.replace('            message: self.message,', '            msg: self.msg,')

# 4. 文档注释中的 message → msg
c = c.replace('/// assert_eq!(resp.message, "ok");', '/// assert_eq!(resp.msg, "ok");')
c = c.replace('//   成功：{ "code": 0, "message": "ok", "data": <T> }', '//   成功：{ "code": 0, "msg": "ok", "data": <T> }')
c = c.replace('//   失败：{ "code": <非0>, "message": "<错误描述>", "data": null }', '//   失败：{ "code": <非0>, "msg": "<错误描述>", "data": null }')
c = c.replace('    /// 人类可读消息', '    /// 人类可读消息（精简字段名 msg）')

# 5. error 函数的参数名保持 message（参数名不影响序列化），但可以改为 msg_param 以避免混淆
# 保持参数名 message 不变，因为它是函数参数，不是结构体字段

with open(fpath, 'w', encoding='utf-8', newline='') as f:
    f.write(c)
print('mox-api-protocol: message -> msg done')
