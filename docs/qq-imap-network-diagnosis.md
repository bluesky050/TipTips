# QQ IMAP TLS 线路诊断

## 结论

当前 imap.qq.com:993 解析进入代理 Fake-IP 地址。该线路在 TLS ClientHello 前返回明文 IMAP 欢迎消息，导致客户端报告协议帧错误；错误发生在账号登录之前。

公开 DNS 查询取得的两个 QQ 地址均能通过主机名为 imap.qq.com 的 TLS 证书验证并读到加密 IMAP 欢迎消息。地址仅用于诊断，不应硬编码到程序或 hosts 文件。

## 最小修复范围

在 Clash Verge 的持久覆盖配置中，仅对 imap.qq.com 设置 DIRECT，并将该域名加入 Fake-IP 排除列表。保留原有规则、DNS过滤模式和其他域名的代理行为；不要编辑或打印节点密钥、订阅地址或外部控制器密钥。

若当前 Fake-IP 过滤模式为默认 blacklist，可合并以下条目（不要覆盖已有列表）：

```yaml
dns:
  fake-ip-filter:
    - imap.qq.com

rules:
  - DOMAIN,imap.qq.com,DIRECT
```

若现有过滤模式为 whitelist 或 rule，需要按该模式追加等效规则。DIRECT 条目应置于通用规则之前。

## 验证

重新加载代理覆盖配置，清除该域名的旧 Fake-IP 映射或 DNS 缓存，关闭旧 IMAP 连接后重试。先做无凭据 TLS 握手，验证正常后再使用应用的连接测试。全程保持服务器证书和主机名校验，不以关闭校验作为修复。

官方参考：

- https://wiki.metacubex.one/config/dns/
- https://wiki.metacubex.one/config/rules/
