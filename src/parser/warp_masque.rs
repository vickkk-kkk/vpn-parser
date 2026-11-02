//! Desktop WARP registration effects; all import syntax is shared with mobile.
use crate::models::Node;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use ring::rand::SecureRandom;
use ring::signature::KeyPair;
use serde_json::{json, Value};
pub use vp_client_core::parser::warp_masque::{WARP_MASQUE_V4, WARP_MASQUE_V6, WARP_MASQUE_PORTS,
OFFICIAL_SNI, SNI_NODE_IP, SNI_NODE_PORT, pem_to_b64, node_name, build_masque_uri, rebuild_masque_uri};
pub fn parse_warp_masque_json(content: &str, source_id: Option<&str>) -> Option<Vec<Node>> {
vp_client_core::parser::warp_masque::parse_warp_masque_json(content, source_id, &mut super::DesktopMetadata)
}
pub fn parse_masque_uri(content: &str, source_id: Option<&str>) -> Option<Node> {
vp_client_core::parser::warp_masque::parse_masque_uri(content, source_id, &mut super::DesktopMetadata)
}
#[allow(clippy::too_many_arguments)]
fn create_masque_node(name: &str, server: &str, port: u16, priv_key: &str, pub_key: &str,
ipv4: &str, ipv6: &str, sni: Option<&str>, source_id: Option<&str>) -> Node {
vp_client_core::parser::warp_masque::create_masque_node(name, server, port, priv_key, pub_key,
ipv4, ipv6, sni, source_id, &mut super::DesktopMetadata)
}
/// 生成标准 P-256 (secp256r1) 密钥对，返回 (spki_base64, sec1_base64)
pub fn generate_p256_keypair() -> anyhow::Result<(String, String)> {
    let rng = ring::rand::SystemRandom::new();
    let pkcs8_doc = ring::signature::EcdsaKeyPair::generate_pkcs8(
        &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
        &rng,
    )
    .map_err(|e| anyhow::anyhow!("生成 P-256 密钥对失败: {e}"))?;

    let key_pair = ring::signature::EcdsaKeyPair::from_pkcs8(
        &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
        pkcs8_doc.as_ref(),
        &rng,
    )
    .map_err(|e| anyhow::anyhow!("解析 P-256 密钥对失败: {e}"))?;

    let pubkey_bytes = key_pair.public_key().as_ref(); // 65 字节未压缩公钥 (0x04 || X || Y)

    // SPKI for P-256 (91 字节: 26 字节固定算法头 + 65 字节公钥)
    let mut spki = Vec::with_capacity(91);
    spki.extend_from_slice(&[
        0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08,
        0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
    ]);
    spki.extend_from_slice(pubkey_bytes);
    let spki_b64 = B64.encode(&spki);

    // 从 PKCS#8 DER 中定位 32 字节私钥标量 (OCTET STRING 0x04 0x20)
    let der = pkcs8_doc.as_ref();
    let pos = der
        .windows(2)
        .position(|w| w == [0x04, 0x20])
        .ok_or_else(|| anyhow::anyhow!("未能从 PKCS#8 中定位私钥标量"))?;
    let raw_priv = &der[pos + 2..pos + 2 + 32];

    // 构建符合 RFC 5915 带 prime256v1 曲线参数的完整 SEC1 结构（供 Mihomo 完美加载）
    let oid = [0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07];
    let mut body = Vec::with_capacity(120);
    body.extend_from_slice(&[0x02, 0x01, 0x01]); // version = 1
    body.extend_from_slice(&[0x04, 0x20]);       // OCTET STRING (32)
    body.extend_from_slice(raw_priv);
    body.push(0xa0);                             // [0] namedCurve
    body.push(oid.len() as u8);
    body.extend_from_slice(&oid);
    body.extend_from_slice(&[0xa1, 0x44, 0x03, 0x42, 0x00]); // [1] publicKey BIT STRING
    body.extend_from_slice(pubkey_bytes);

    let mut sec1 = Vec::with_capacity(128);
    sec1.push(0x30);
    sec1.push(body.len() as u8);
    sec1.extend_from_slice(&body);

    let sec1_b64 = B64.encode(&sec1);
    Ok((spki_b64, sec1_b64))
}

/// 原生向 Cloudflare 注册新 WARP 设备并登记 MASQUE 隧道，生成全部 57 个节点
pub async fn register_warp_masque_native(proxy_url: Option<&str>) -> anyhow::Result<Vec<Node>> {
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15));
    if let Some(proxy) = proxy_url {
        builder = builder.proxy(reqwest::Proxy::all(proxy)?);
    }
    let client = builder.build()?;

    let (spki, sec1) = generate_p256_keypair()?;

    let mut rand_bytes = [0u8; 32];
    ring::rand::SystemRandom::new()
        .fill(&mut rand_bytes)
        .map_err(|e| anyhow::anyhow!("随机数生成失败: {e}"))?;
    let rand_b64 = B64.encode(&rand_bytes);

    let mut serial_bytes = [0u8; 8];
    ring::rand::SystemRandom::new()
        .fill(&mut serial_bytes)
        .map_err(|e| anyhow::anyhow!("随机数生成失败: {e}"))?;
    let serial_hex: String = serial_bytes.iter().map(|b| format!("{b:02x}")).collect();

    let now_str = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, false);

    // 1. POST /reg 注册设备
    let reg_body = json!({
        "key": rand_b64,
        "install_id": "",
        "fcm_token": "",
        "tos": now_str,
        "model": "PC",
        "serial_number": serial_hex,
        "os_version": "",
        "key_type": "curve25519",
        "tunnel_type": "wireguard",
        "locale": "en-US",
    });

    let resp = client
        .post("https://api.cloudflareclient.com/v0a4471/reg")
        .header("User-Agent", "WARP for Android")
        .header("CF-Client-Version", "a-6.35-4471")
        .header("Content-Type", "application/json; charset=UTF-8")
        .json(&reg_body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let err_text = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("WARP 设备注册失败: {err_text}"));
    }

    let acc: serde_json::Value = resp.json().await?;
    let acc_id = acc
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("未获取到账号 ID"))?;
    let token = acc
        .get("token")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("未获取到注册 Token"))?;

    // 2. PATCH /reg/{id} 挂载 MASQUE 公钥
    let patch_body = json!({
        "key": spki,
        "key_type": "secp256r1",
        "tunnel_type": "masque",
        "name": "vpn-parser",
    });

    let patch_resp = client
        .patch(format!("https://api.cloudflareclient.com/v0a4471/reg/{acc_id}"))
        .header("User-Agent", "WARP for Android")
        .header("CF-Client-Version", "a-6.35-4471")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json; charset=UTF-8")
        .json(&patch_body)
        .send()
        .await?;

    if !patch_resp.status().is_success() {
        let err_text = patch_resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("WARP MASQUE 登记失败: {err_text}"));
    }

    let up: serde_json::Value = patch_resp.json().await?;

    let raw_peer_pub = up
        .pointer("/config/peers/0/public_key")
        .and_then(Value::as_str)
        .unwrap_or("");
    let peer_pub = pem_to_b64(raw_peer_pub);
    if peer_pub.is_empty() {
        return Err(anyhow::anyhow!("未能获取到对端公钥"));
    }

    let ipv4 = up
        .pointer("/config/interface/addresses/v4")
        .or_else(|| acc.pointer("/config/interface/addresses/v4"))
        .and_then(Value::as_str)
        .unwrap_or("172.16.0.2");

    let ipv6 = up
        .pointer("/config/interface/addresses/v6")
        .or_else(|| acc.pointer("/config/interface/addresses/v6"))
        .and_then(Value::as_str)
        .unwrap_or("2606:4700:110:8878:e7b0:f0eb:a03d:8524");

    // 3. 展开 57 个节点
    let mut nodes = Vec::new();
    for &ip in WARP_MASQUE_V4 {
        for &port in WARP_MASQUE_PORTS {
            let name = node_name(ip, port);
            nodes.push(create_masque_node(
                &name,
                ip,
                port,
                &sec1,
                &peer_pub,
                ipv4,
                ipv6,
                None,
                Some("src_warp_native"),
            ));
        }
    }
    for &ip in WARP_MASQUE_V6 {
        for &port in WARP_MASQUE_PORTS {
            let name = node_name(ip, port);
            nodes.push(create_masque_node(
                &name,
                ip,
                port,
                &sec1,
                &peer_pub,
                ipv4,
                ipv6,
                None,
                Some("src_warp_native"),
            ));
        }
    }
    nodes.push(create_masque_node(
        "WARP-官方域名",
        SNI_NODE_IP,
        SNI_NODE_PORT,
        &sec1,
        &peer_pub,
        ipv4,
        ipv6,
        Some(OFFICIAL_SNI),
        Some("src_warp_native"),
    ));

    Ok(nodes)
}


#[cfg(test)]
mod tests {
use super::*;
    #[test]
    fn rebuilds_masque_uri_and_clash_proxy() {
        let uri = "masque://162.159.198.1:443?publicKey=testpub&privateKey=testpriv&ip=172.16.0.2#WARP-198.1-443";
        let node = parse_masque_uri(uri, None).expect("should parse");
        let rebuilt = rebuild_masque_uri(&node.config, &node).expect("rebuilt uri");
        assert!(rebuilt.starts_with("masque://162.159.198.1:443?"));
        assert!(rebuilt.contains("publicKey=testpub"));

        let clash_proxies = crate::export::nodes_to_clash_proxies(&[node]);
        assert_eq!(clash_proxies.len(), 1);
        assert_eq!(clash_proxies[0]["type"], "masque");
        assert_eq!(clash_proxies[0]["server"], "162.159.198.1");
    }

    #[test]
    fn test_generate_p256_keypair_returns_valid_spki_and_sec1() {
        let (spki, sec1) = generate_p256_keypair().expect("generate p256 keypair");
        assert!(!spki.is_empty());
        assert!(!sec1.is_empty());
        // SPKI should be 91 bytes (base64 length around 124)
        let spki_bytes = B64.decode(&spki).expect("decode spki");
        assert_eq!(spki_bytes.len(), 91);
        // SEC1 should be 121 bytes (base64 length around 164)
        let sec1_bytes = B64.decode(&sec1).expect("decode sec1");
        assert_eq!(sec1_bytes.len(), 121);
    }
}
