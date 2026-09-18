use azalea_client::PacketProcessingBudget;
use azalea_client::test_utils::prelude::*;
use azalea_entity::metadata::Cow;
use azalea_core::{delta::PositionDelta8, entity_id::MinecraftEntityId, position::ChunkPos};
use azalea_protocol::packets::{
    ConnectionProtocol,
    game::ClientboundMoveEntityPos,
};
use azalea_registry::builtin::EntityKind;
use azalea_entity::Position;
use bevy_ecs::query::With;

#[test]
fn test_moves_coalesce_to_latest_under_budget_exhaustion() {
    let _lock = init();

    let mut simulation = Simulation::new(ConnectionProtocol::Game);
    simulation.receive_packet(default_login_packet());
    simulation.receive_packet(make_basic_empty_chunk(ChunkPos::new(0, 0), (384 + 64) / 16));
    simulation.tick();
    // spawn a cow at (0.5, 64., 0.5)
    simulation.receive_packet(make_basic_add_entity(EntityKind::Cow, 123, (0.5, 64., 0.5)));
    simulation.tick();

    // forcibly exhaust the budget so the exhaustion path (last-write-wins
    // move coalescing) is exercised deterministically
    simulation.app.world_mut().insert_resource(PacketProcessingBudget {
        max_ms_per_cycle: 20.0,
        max_packets_per_bot: 50,
        fake_exhausted: true,
    });

    // warm up the entity's UpdatesReceived counter
    simulation.receive_packet(ClientboundMoveEntityPos {
        entity_id: MinecraftEntityId(123),
        delta: PositionDelta8 { xa: 0, ya: 0, za: 4096 },
        on_ground: true,
    });
    simulation.tick();

    // two moves for the same entity under Mini budget along; only the LAST one may apply
    simulation.receive_packet(ClientboundMoveEntityPos {
        entity_id: MinecraftEntityId(123),
        delta: PositionDelta8 { xa: 4096, ya: 0, za: 0 },
        on_ground: true,
    });
    simulation.receive_packet(ClientboundMoveEntityPos {
        entity_id: MinecraftEntityId(123),
        delta: PositionDelta8 { xa: 0, ya: 0, za: 8192 },
        on_ground: true,
    });
    simulation.tick();

    let mut cow_query = simulation.app.world_mut().query_filtered::<&Position, With<Cow>>();
    let position = *cow_query.iter(simulation.app.world()).next().unwrap();
    // warmup applied za=+1; of the two burst moves only the last (za +=2) is
    // applied, so x stays 0.5
    assert_eq!(*position, azalea_core::position::Vec3::new(0.5, 64., 3.5));
}
