import NetworkExtension

/// Fail-closed extension boundary, NOT a packet engine or a functioning VPN.
final class PacketTunnelProvider: NEPacketTunnelProvider {
    override func startTunnel(options: [String: NSObject]?,
                              completionHandler: @escaping (Error?) -> Void) {
        // Never call setTunnelNetworkSettings or return nil before a real engine,
        // packetFlow pump, DNS/routing plan and cancellation path are integrated.
        completionHandler(NSError(
            domain: "org.vpnparser.client.PacketTunnel", code: 1,
            userInfo: [NSLocalizedDescriptionKey:
                "Tunnel unavailable: no embedded packet engine is bound."]
        ))
    }

    override func stopTunnel(with reason: NEProviderStopReason,
                             completionHandler: @escaping () -> Void) {
        completionHandler() // No descriptor, settings or engine was created.
    }
}
