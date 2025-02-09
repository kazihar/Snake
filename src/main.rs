use std::time::Duration;

use bevy::{
    prelude::*,
    time::common_conditions::on_timer,
    window::{PrimaryWindow, WindowResolution},
};
use rand::random;

const SNAKE_HEAD_COLOR: Color = Color::linear_rgb(0.7, 0.7, 0.7);
const SNAKE_SEGMENT_COLOR: Color = Color::linear_rgb(0.3, 0.3, 0.3);
const FOOD_COLOR: Color = Color::linear_rgb(1.0, 0.0, 1.0);

const ARENA_WIDTH: u32 = 10;
const ARENA_HEIGHT: u32 = 10;

#[derive(PartialEq, Clone, Copy)]
enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}

#[derive(Component)]
struct SnakeHead {
    direction: Direction,
}

#[derive(Component)]
struct SnakeSegment;

#[derive(Default, Resource)]
struct SnakeSegments(Vec<Entity>);
#[derive(Default, Resource)]
struct LastTailPosition(Option<Position>);

#[derive(Event)]
struct GrowthEvent;
#[derive(Event)]
struct GameOverEvent;

#[derive(Component)]
struct Food;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Component)]
struct Size {
    width: f32,
    height: f32,
}

impl Size {
    pub fn square(x: f32) -> Self {
        Self {
            width: x,
            height: x,
        }
    }
}

fn main() {
    App::new()
        .add_systems(Startup, setup_camera)
        .add_systems(Startup, spawn_snake)
        .add_systems(FixedUpdate, snake_movement_input.before(snake_movement))
        .add_systems(
            FixedUpdate,
            (
                snake_movement,
                snake_eating.after(snake_movement),
                snake_growth.after(snake_eating),
            )
                .run_if(on_timer(Duration::from_millis(150))),
        )
        .add_systems(FixedUpdate, game_over.after(snake_movement))
        .add_systems(PostUpdate, (position_translation, size_scaling))
        .add_systems(
            FixedUpdate,
            food_spawner.run_if(on_timer(Duration::from_secs(1))),
        )
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Snake".to_string(),
                resizable: false,
                resolution: WindowResolution::new(500.0, 500.0),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .insert_resource(ClearColor(Color::linear_rgb(0.0, 0.0, 0.0)))
        .insert_resource(SnakeSegments::default())
        .insert_resource(LastTailPosition::default())
        .add_event::<GrowthEvent>()
        .add_event::<GameOverEvent>()
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d::default());
}

fn spawn_snake(mut commands: Commands, mut segments: ResMut<SnakeSegments>) {
    *segments = SnakeSegments(vec![
        commands
            .spawn(Sprite {
                color: SNAKE_HEAD_COLOR,
                ..Default::default()
            })
            .insert(SnakeHead {
                direction: Direction::Up,
            })
            .insert(SnakeSegment)
            .insert(Position { x: 3, y: 3 })
            .insert(Size::square(0.8))
            .id(),
        spawn_segment(commands, Position { x: 3, y: 3 }),
    ]);
}

fn size_scaling(
    windows: Query<&mut Window, With<PrimaryWindow>>,
    mut query: Query<(&Size, &mut Transform)>,
) {
    let window = windows.single();
    for (sprite_size, mut transform) in query.iter_mut() {
        transform.scale = Vec3::new(
            sprite_size.width / ARENA_WIDTH as f32 * window.width() as f32,
            sprite_size.height / ARENA_HEIGHT as f32 * window.height() as f32,
            1.0,
        );
    }
}

fn position_translation(
    windows: Query<&mut Window, With<PrimaryWindow>>,
    mut query: Query<(&Position, &mut Transform)>,
) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }
    let window = windows.single();
    for (pos, mut transform) in query.iter_mut() {
        transform.translation = Vec3::new(
            convert(pos.x as f32, window.width() as f32, ARENA_WIDTH as f32),
            convert(pos.y as f32, window.height() as f32, ARENA_HEIGHT as f32),
            0.0,
        );
    }
}

fn snake_movement_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut heads: Query<&mut SnakeHead>,
) {
    if let Some(mut head) = heads.iter_mut().next() {
        let dir: Direction = if keyboard_input.pressed(KeyCode::ArrowLeft) {
            Direction::Left
        } else if keyboard_input.pressed(KeyCode::ArrowDown) {
            Direction::Down
        } else if keyboard_input.pressed(KeyCode::ArrowUp) {
            Direction::Up
        } else if keyboard_input.pressed(KeyCode::ArrowRight) {
            Direction::Right
        } else {
            head.direction
        };
        if dir != head.direction.opposite() {
            head.direction = dir;
        }
    }
}

fn snake_movement(
    segments: ResMut<SnakeSegments>,
    mut heads: Query<(Entity, &SnakeHead)>,
    mut last_tail_position: ResMut<LastTailPosition>,
    mut positions: Query<&mut Position>,
    mut game_over_writer: EventWriter<GameOverEvent>,
) {
    if let Some((head_entity, head)) = heads.iter_mut().next() {
        let segment_positions = segments
            .0
            .iter()
            .map(|e| *positions.get_mut(*e).unwrap())
            .collect::<Vec<Position>>();
        let mut head_pos = positions.get_mut(head_entity).unwrap();
        match &head.direction {
            Direction::Left => {
                head_pos.x -= 1;
            }
            Direction::Right => {
                head_pos.x += 1;
            }
            Direction::Up => {
                head_pos.y += 1;
            }
            Direction::Down => {
                head_pos.y -= 1;
            }
        };

        if head_pos.x < 0
            || head_pos.y < 0
            || head_pos.x as u32 >= ARENA_WIDTH
            || head_pos.y as u32 >= ARENA_HEIGHT
        {
            game_over_writer.send(GameOverEvent);
        }

        if segment_positions.contains(&head_pos) {
            game_over_writer.send(GameOverEvent);
        }

        segment_positions
            .iter()
            .zip(segments.0.iter().skip(1))
            .for_each(|(pos, segment)| *positions.get_mut(*segment).unwrap() = *pos);
        *last_tail_position = LastTailPosition(Some(*segment_positions.last().unwrap()));
    }
}

fn spawn_segment(mut commands: Commands, position: Position) -> Entity {
    commands
        .spawn(Sprite {
            color: SNAKE_SEGMENT_COLOR,
            ..Default::default()
        })
        .insert(SnakeSegment)
        .insert(position)
        .insert(Size::square(0.65))
        .id()
}

fn food_spawner(mut commands: Commands, head_positions: Query<&Position, With<SnakeHead>>) {
    let mut x;
    let mut y;
    let mut valid_position;

    loop {
        x = (random::<f32>() * ARENA_WIDTH as f32) as i32;
        y = (random::<f32>() * ARENA_HEIGHT as f32) as i32;
        valid_position = true;

        for head_pos in head_positions.iter() {
            if head_pos.x == x && head_pos.y == y {
                valid_position = false;
                break;
            }
        }

        if valid_position {
            break;
        }
    }

    commands
        .spawn(Sprite {
            color: FOOD_COLOR,
            ..Default::default()
        })
        .insert(Food)
        .insert(Position { x, y })
        .insert(Size::square(0.8));
}

fn snake_eating(
    mut commands: Commands,
    mut growth_writer: EventWriter<GrowthEvent>,
    food_positions: Query<(Entity, &Position), With<Food>>,
    head_positions: Query<&Position, With<SnakeHead>>,
) {
    for head_pos in head_positions.iter() {
        for (ent, food_pos) in food_positions.iter() {
            if food_pos == head_pos {
                commands.entity(ent).despawn();
                growth_writer.send(GrowthEvent);
            }
        }
    }
}

fn snake_growth(
    commands: Commands,
    last_tail_position: Res<LastTailPosition>,
    mut segments: ResMut<SnakeSegments>,
    growth_reader: EventReader<GrowthEvent>,
) {
    if !growth_reader.is_empty() {
        segments
            .0
            .push(spawn_segment(commands, last_tail_position.0.unwrap()))
    }
}

fn game_over(
    mut commands: Commands,
    reader: EventReader<GameOverEvent>,
    segments_res: ResMut<SnakeSegments>,
    food: Query<Entity, With<Food>>,
    segments: Query<Entity, With<SnakeSegment>>,
) {
    if !reader.is_empty() {
        for ent in food.iter().chain(segments.iter()) {
            commands.entity(ent).despawn();
        }
        spawn_snake(commands, segments_res);
    }
}
