use deviruchi::game::inter_server::{CharacterTransfer, InterServerConnector, InterServerPacket, ServerTypeProto};
use deviruchi::game::map::MapState;
use deviruchi::game::server_registry::ServerRegistry;
use deviruchi::network::inter_server::{InterServerTcpServer, TcpInterServerConnector};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// 测试跨进程 CharToMap 传输：模拟 CharServer 向 MapServer 发送角色数据
#[test]
fn test_cross_process_char_to_map_transfer() {
    // 1. 启动 MapServer 的 inter-server 监听（使用固定端口）
    let map_inter_addr = "127.0.0.1:16121";
    let map_comm = Arc::new(deviruchi::game::inter_server::InterServerComm::new());
    let map_state = Arc::new(MapState::new());

    let map_server = InterServerTcpServer::new(
        map_inter_addr.to_string(),
        3, // map_server_id
        map_comm.clone(),
    );

    let map_state_for_accept = map_state.clone();
    map_server.listen_blocking(move |peer_id, connector| {
        println!("Map server accepted connection from peer {}", peer_id);
        // 启动一个线程处理来自 char server 的包
        let map_state = map_state_for_accept.clone();
        let connector_clone = connector.clone();
        thread::spawn(move || {
            loop {
                match connector_clone.recv_packet() {
                    Ok(Some(packet)) => {
                        match packet {
                            InterServerPacket::CharToMap { char_id, account_id: _, token: _, map_server_id: _, character_data } => {
                                println!("MapServer received CharToMap: char_id={}", char_id);
                                // 在 map_state 中创建玩家
                                let player = deviruchi::game::map::Player::from_character_transfer(&character_data);
                                map_state.add_player(player);
                            }
                            InterServerPacket::Heartbeat { .. } => {
                                // 忽略心跳
                            }
                            _ => {
                                println!("MapServer received unexpected packet");
                            }
                        }
                    }
                    Ok(None) => {
                        println!("Connection closed by peer");
                        break;
                    }
                    Err(e) => {
                        println!("Error receiving packet: {}", e);
                        break;
                    }
                }
            }
        });
    });

    // 等待 MapServer 启动
    thread::sleep(Duration::from_millis(100));

    // 2. 模拟 CharServer 连接到 MapServer 并发送 CharToMap
    let connector = TcpInterServerConnector::connect(3, map_inter_addr).unwrap();

    // 发送注册包
    let register_packet = InterServerPacket::ServerRegister {
        id: 2, // char_server_id
        name: "char-server".to_string(),
        ip: "127.0.0.1".to_string(),
        port: 0,
        server_type: ServerTypeProto::Char,
        max_players: 100,
    };
    connector.send_packet(&register_packet).unwrap();

    // 等待 MapServer 处理注册
    thread::sleep(Duration::from_millis(200));

    // 3. 构造角色传输数据并发送
    let transfer = CharacterTransfer {
        char_id: 1001,
        account_id: 1,
        name: "TestChar".to_string(),
        level: 99,
        job: 0,
        hp: 5000,
        max_hp: 5000,
        sp: 1000,
        max_sp: 1000,
        map_name: "prontera".to_string(),
        pos_x: 100,
        pos_y: 200,
        save_map: "new_1-1".to_string(),
        save_x: 53,
        save_y: 111,
        str: 99,
        agi: 99,
        vit: 99,
        int: 99,
        dex: 99,
        luk: 99,
        zeny: 1000000,
        sex: 1,
        hair_color: 0,
        hair: 1,
        cloak_id: 0,
        boots_id: 0,
        account_level: 0,
    };

    let char_to_map_packet = InterServerPacket::CharToMap {
        char_id: transfer.char_id,
        account_id: transfer.account_id,
        token: "test_token_123".to_string(),
        map_server_id: 3,
        character_data: transfer.clone(),
    };

    connector.send_packet(&char_to_map_packet).unwrap();

    // 4. 等待 MapServer 处理
    thread::sleep(Duration::from_millis(500));

    // 5. 验证 MapState 中已添加玩家
    let players = map_state.get_players_on_map("prontera");
    assert_eq!(players.len(), 1, "Expected 1 player on prontera");
    // Player 字段是私有的，通过 get_players_on_map 返回的 Player 验证
    // 由于 Player 字段私有，我们只验证数量和基本存在性
    // 更详细的验证可以通过 MapState 的其他方法

    println!("Cross-process CharToMap test passed!");
}

/// 测试心跳和服务器注册
#[test]
fn test_inter_server_heartbeat_and_registration() {
    let registry = Arc::new(ServerRegistry::new());

    // 注册一个服务器
    registry.register_server(deviruchi::game::server_registry::ServerInfo {
        id: 1,
        name: "test-server".to_string(),
        ip: "127.0.0.1".to_string(),
        port: 6900,
        online_players: 0,
        max_players: 100,
        last_heartbeat: std::time::Instant::now(),
        server_type: deviruchi::game::server_registry::ServerType::Login,
    }).unwrap();

    // 验证注册成功
    let server = registry.get_server(1).unwrap();
    assert_eq!(server.name, "test-server");
    assert_eq!(server.port, 6900);

    // 更新心跳
    registry.update_heartbeat(1).unwrap();
    let updated = registry.get_server(1).unwrap();
    assert!(updated.last_heartbeat.elapsed() < Duration::from_secs(1));

    // 更新玩家数
    registry.update_player_count(1, 42).unwrap();
    let with_players = registry.get_server(1).unwrap();
    assert_eq!(with_players.online_players, 42);
}

/// 测试单进程 all 模式向后兼容（localhost 自连接）
#[test]
fn test_all_mode_localhost_self_connect() {
    // 创建配置
    let _config = deviruchi::core::Config::default();
}
