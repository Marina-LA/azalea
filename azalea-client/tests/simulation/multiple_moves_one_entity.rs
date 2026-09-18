use azalea_client::test_utils::prelude::*;
use azalea_entity::metadata::Cow;
use azalea_core::{
    delta::PositionDelta8,
    entity_id::MinecraftEntityId,
    position::ChunkPos,
};
use azalea_protocol::packets::{
    ConnectionProtocol,
    game::ClientboundMoveEntityPos,
};
use azalea_registry::builtin::EntityKind;
use azalea_entity::Position;
use bevy_ecs::query::With;

#[test]
fn test_applies_multiple_moves_for_one_entity_in_one_cycle() {
    let _lock = init();

    let mut simulation = Simulation::new(ConnectionProtocol::Game);
    simulation.receive_packet(default_login_packet());
    simulation.receive_packet(make_basic_empty_chunk(ChunkPos::new(0, 0), (384 + 64) / 16));
    simulation.tick();
    // spawn a cow at (0.5, 64., 0.5)
    simulation.receive_packet(make_basic_add_entity(EntityKind::Cow, 123, (0.5, 64., 0.5)));
    simulation.tick();

    // warm up the entity's UpdatesReceived counter (a fresh entity applies
    // unconditionally until the component first materializes)
    simulation.receive_packet(ClientboundMoveEntityPos {
        entity_id: MinecraftEntityId(123),
        delta: PositionDelta8 { xa: 0, ya: 0, za: 4096 },
        on_ground: true,
    });
    simulation.tick();

    // two moves in a single read cycle for the same entity must both apply
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
    assert_eq!(*position, azalea_core::position::Vec3::new(1.5, 64., 3.5));
}
