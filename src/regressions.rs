//! Application acceptance setups; all transport and contacts use the engine.
use super::*;
use serde_json::{Value, json};

impl WorldGame {
    pub fn prepare_water_occlusion_scenario(&mut self, open: bool) {
        let keep = [
            self.ids.player,
            self.ids.camp_logs[0],
            self.ids.camp_logs[1],
        ];
        let remove: Vec<_> = self
            .world
            .entities()
            .map(|(&id, _)| id)
            .filter(|id| !keep.contains(id))
            .collect();
        for id in remove {
            self.world.remove(id);
        }
        let jet = self.current_water_jet_for_turns(WATER_BURST_TURNS).unwrap();
        let samples = water_jet_samples(jet);
        for (id, sample, name) in [
            (keep[1], 4, "front receiver"),
            (keep[2], 10, "rear receiver"),
        ] {
            let entity = self.world.entity_mut(id).unwrap();
            entity.name = Some(name.into());
            entity.transform = Transform {
                position: samples[sample].center,
                rotation: Quat::from_rotation_z(FRAC_PI_2),
                scale: Vec3::splat(2.0),
            };
            // Match the authored log's visible length and thickness.
            entity.collider = Some(Collider::CapsuleY {
                radius: 0.19,
                half_height: 0.475,
            });
            entity.body = Some(Body::static_body());
            entity.reactive_state = Some(ReactiveState::new(24.0, 0.0, 1.0));
        }
        if open {
            self.world.entity_mut(keep[1]).unwrap().transform.position.x += 4.0;
        }
        self.orbit_yaw = 1.05;
    }

    /// Inspect the actual composed beams and animated model, alongside water
    /// delivery and velocity. The caller also captures the same scene as a PNG.
    pub fn verify_regression_scenario(&self, name: &str) -> Result<Option<Value>> {
        if name == "controls-stop" {
            let player = self.world.entity(self.ids.player).unwrap();
            let assets = self.assets.as_ref().context("character assets missing")?;
            let pose = self
                .current_explorer_pose(assets, self.presentation_time)
                .unwrap();
            ensure!(
                self.controls.open && player.body.unwrap().linear_velocity == Vec3::ZERO,
                "controls did not stop the player body"
            );
            ensure!(
                pose.anim.clip == assets.explorer_animations.idle,
                "stopped player still walks"
            );
            return Ok(Some(
                json!({"player_speed": 0.0, "player_animation": "Idle"}),
            ));
        }
        if !matches!(name, "water-receiver-blocked" | "water-receiver-open") {
            return Ok(None);
        }
        let jet = self
            .current_water_jet_for_turns(self.water_pose_turns)
            .context("no water jet")?;
        let path = &water_jet_emissions(jet, self.ids.player)[0];
        let limit = self.world.water_path_distance(path);
        let visible: f32 = self
            .scene
            .beams
            .iter()
            .filter(|b| b.color[0] == 0.12)
            .map(|b| b.a.distance(b.b))
            .sum();
        ensure!(
            visible > 0.0 && (visible - limit).abs() < 1e-5,
            "visible water length {visible} differs from transport {limit}"
        );
        let receipt = self.water_receipt();
        let delivered = |id: EntityId| {
            receipt
                .delivered_by_entity
                .iter()
                .find(|dose| dose.entity == id.0)
                .map_or(0.0, |dose| dose.amount)
        };
        let front = delivered(self.ids.camp_logs[0]);
        let rear = delivered(self.ids.camp_logs[1]);
        if name == "water-receiver-blocked" {
            ensure!(
                front > 0.0 && rear == 0.0 && limit < 1.2,
                "front receiver must stop water: front={front}, rear={rear}, length={limit}"
            );
        } else {
            ensure!(
                front == 0.0 && rear > 0.0 && limit > 1.2,
                "moving receiver must open water path: front={front}, rear={rear}, length={limit}"
            );
        }
        Ok(Some(
            json!({"water_visible_length": visible, "water_impact_distance": limit,
            "front_delivered": front, "rear_delivered": rear}),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pocket3d_world::WaterEmission;

    #[test]
    fn seed_seven_grass_does_not_receive_water_above_its_capsule() {
        let mut game = WorldGame::new(7);
        let (&id, grass) = game
            .world
            .entities()
            .find(|(_, e)| e.name.as_deref() == Some("grass tuft 2"))
            .unwrap();
        let center = grass.transform.position;
        let mut control = WorldGame::new(7);
        control.world.step(&control.environment);
        let emission = WaterEmission {
            points: vec![
                center + Vec3::new(0.0, 0.451, 1.0),
                center + Vec3::new(0.0, 0.451, -1.0),
            ],
            amount: 0.1,
            temperature_c: 24.0,
            source: None,
        };
        game.world.queue_interaction(Interaction::Water(emission));
        let report = game.world.step(&game.environment);
        assert!(!report.transport.water.iter().any(|dose| dose.target == id));
        assert_eq!(
            game.world
                .entity(id)
                .unwrap()
                .reactive_state
                .unwrap()
                .moisture,
            control
                .world
                .entity(id)
                .unwrap()
                .reactive_state
                .unwrap()
                .moisture
        );
    }
}
