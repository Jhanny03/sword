use std::time::{Duration, Instant};
use iced::{Application, Column, Container, Element, Length, Point, Rectangle, Renderer, Settings, time, window};
use iced_native::{Command, Layout, renderer, Subscription, Widget};
use iced_native::layout::{Limits, Node};
use iced_native::renderer::Style;
use crate::AppMessage::Network;
use crate::network::NetworkMessage;

fn main() -> iced::Result {
    println!("Init");
    App::run(Settings{
        window: window::Settings{
            position: window::Position::Centered,
            resizable: false,
            decorations: true,
            transparent: false,
            always_on_top: false,
            icon: None,
            ..window::Settings::default()
        },
        antialiasing: true,
        exit_on_close_request: true,
        ..Settings::default()
    })
}

struct App{
    network: network::Network,
}

#[derive(Debug)]
enum AppMessage{
    Tick(Instant),
    Network(NetworkMessage),
}

impl iced::Application for App{
    type Executor = iced::executor::Default;
    type Message = AppMessage;
    type Flags = ();

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (Self{
            network: network::Network::new(),
        }, Command::none())
    }

    fn title(&self) -> String {
        String::from("Sword")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            AppMessage::Tick(_) => {}
            AppMessage::Network(_) => {}
        }
        Command::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        time::every(Duration::from_millis(1000 / 100))
            .map(AppMessage::Tick)
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        let content = Column::new()
            .push(self.network
                .view()
                .map(move |message| Network(message))
            );
        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

mod network{
    use iced::{Color, Length, mouse, Point, Rectangle, Size, Vector};
    use iced::keyboard::KeyCode::S;
    use iced_graphics::canvas::{Cache, Cursor, Event, event, Fill, FillRule, Frame, Geometry, Path, Stroke};
    use iced_native::event::Status;

    pub struct Network{
        nodes_cache: Cache,
        interaction: Interaction,
        translation: Vector,
        scaling: f32,
        nodes: Vec<Node>
    }

    #[derive(Debug)]
    pub enum NetworkMessage{
        Update,
    }

    enum Interaction{
        None,
        PanningScreen { translation: iced::Vector, start: iced::Point },
        PanningNode { node_id: u32, translation: iced::Vector, start: iced::Point },
    }

    impl Network{
        const MIN_SCALING: f32 = 0.1;
        const MAX_SCALING: f32 = 2.0;

        pub fn new() -> Self{
            let n1 = Node{
                id: 0,
                bounds: Rectangle{
                    x: 0.,
                    y: 0.,
                    width: 100.,
                    height: 100.,
                },
                color: Color::BLACK,
                is_selected: false,
            };
            Network{
                nodes_cache: Default::default(),
                interaction: Interaction::None,
                translation: Default::default(),
                scaling: 1.0,
                nodes: vec![n1]
            }
        }

        pub fn view(&mut self) -> iced::Element<NetworkMessage> {
            iced_graphics::Canvas::new(self)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }

        fn visible_region(&self, size: Size) -> Region {
            let width = size.width / self.scaling;
            let height = size.height / self.scaling;

            Region {
                x: -self.translation.x - width / 2.0,
                y: -self.translation.y - height / 2.0,
                width,
                height,
            }
        }

        fn project(&self, position: Point, size: Size) -> Point {
            let region = self.visible_region(size);

            Point::new(
                position.x / self.scaling + region.x,
                position.y / self.scaling + region.y,
            )
        }

        fn get_node_at_screen(&mut self, position: Point) -> Option<u32>{
            for node in self.nodes.iter_mut() {
                if node.bounds.contains(position){
                    return Some(node.id);
                }
            };
            None
        }

        fn unselect_all_nodes(&mut self){
            for node in self.nodes.iter_mut(){
                node.set_selected(false);
            }
        }
    }

    impl<'a> iced_graphics::canvas::Program<NetworkMessage> for Network{
        fn update(
            &mut self,
            event: Event,
            bounds: Rectangle,
            cursor: Cursor,
        ) -> (event::Status, Option<NetworkMessage>) {

            if let Event::Mouse(mouse::Event::ButtonReleased(_)) = event {
                self.interaction = Interaction::None;
            }

            let cursor_position =
                if let Some(position) = cursor.position_in(&bounds) {
                    position
                } else {
                    return (event::Status::Ignored, None);
                };

            let node_id = self.get_node_at_screen(self.project(cursor_position, bounds.size()));

            match event {
                Event::Mouse(mouse_event) => match mouse_event {
                    mouse::Event::ButtonPressed(button) => {
                        let message = match button {
                            mouse::Button::Left => {
                                match node_id {
                                    Some(id) => {
                                        let node = self.nodes.iter_mut().find(|x| x.id == id);
                                        match node{
                                            Some(n) => {
                                                self.interaction = Interaction::PanningNode {
                                                    node_id: n.id,
                                                    translation: Vector::new(n.bounds.x, n.bounds.y),
                                                    start: cursor_position,
                                                };
                                                n.set_selected(true);
                                            }
                                            None => {
                                                println!("Could not select node with id:{} because \
                                                the node could not be found in the network", id);
                                            }
                                        }
                                    }
                                    None => {
                                        self.unselect_all_nodes();
                                    }
                                }
                                self.nodes_cache.clear();
                                None
                            }
                            mouse::Button::Middle => {
                                self.interaction = Interaction::PanningScreen {
                                    translation: self.translation,
                                    start: cursor_position,
                                };
                                None
                            }
                            _ => None,
                        };
                        (event::Status::Captured, message)
                    }
                    mouse::Event::CursorMoved { .. } => {
                        let message = match self.interaction {
                            Interaction::PanningScreen { translation, start } => {
                                self.translation = translation
                                    + (cursor_position - start)
                                    * (1.0 / self.scaling);
                                self.nodes_cache.clear();
                                None
                            }
                            Interaction::PanningNode {node_id, translation, start } => {
                                let node = self.nodes.iter_mut().find(|x| x.id == node_id);
                                match node {
                                    Some(n) => {
                                        let new_pos = translation
                                            + (cursor_position - start)
                                            * (1.0 / self.scaling);
                                        n.set_new_pos(new_pos);
                                        self.nodes_cache.clear();
                                    }
                                    None => {
                                        println!("Could not pan node with id:{} because \
                                                the node could not be found in the network",
                                                 node_id);
                                    }
                                }
                                None
                            }
                            _ => None,
                        };
                        let event_status = match self.interaction {
                            Interaction::None => event::Status::Ignored,
                            _ => event::Status::Captured,
                        };
                        (event_status, message)
                    }
                    mouse::Event::WheelScrolled { delta } => match delta {
                        mouse::ScrollDelta::Lines { y, .. }
                        | mouse::ScrollDelta::Pixels { y, .. } => {
                            if y < 0.0 && self.scaling > Self::MIN_SCALING
                                || y > 0.0 && self.scaling < Self::MAX_SCALING
                            {
                                let old_scaling = self.scaling;
                                self.scaling = (self.scaling
                                    * (1.0 + y / 30.0))
                                    .max(Self::MIN_SCALING)
                                    .min(Self::MAX_SCALING);

                                if let Some(cursor_to_center) =
                                cursor.position_from(bounds.center())
                                {
                                    let factor = self.scaling - old_scaling;
                                    self.translation = self.translation
                                        - Vector::new(
                                        cursor_to_center.x * factor
                                            / (old_scaling * old_scaling),
                                        cursor_to_center.y * factor
                                            / (old_scaling * old_scaling),
                                    );
                                }
                                self.nodes_cache.clear();
                            }
                            (event::Status::Captured, None)
                        }
                    },
                    _ => (event::Status::Ignored, None),
                },
                _ => (event::Status::Ignored, None),
            }
        }

        fn draw(&self, bounds: Rectangle, cursor: Cursor) -> Vec<Geometry> {
            let center = Vector::new(bounds.width / 2.0, bounds.height / 2.0);
            let mut frame = Frame::new(bounds.size());
            let background = Path::rectangle(Point::ORIGIN, frame.size());
            frame.fill(&background, Color::from_rgb8(0x04, 0x44, 0x48));

            let nodes = self.nodes_cache.draw(bounds.size(), |frame| {
                for node in &self.nodes{
                    frame.translate(center);
                    frame.scale(self.scaling);
                    frame.translate(self.translation);

                    let line_stroke = Stroke{
                        color: Color::WHITE,
                        width: 5.0 * self.scaling,
                        ..Stroke::default()
                    };
                    let line = Path::line(Point::new(0., 0.), Point::new(500.0, 0.0));
                    frame.fill(&line, Color::WHITE);
                    frame.stroke(&line, line_stroke);

                    node.draw(frame, self.scaling);
                }
            });

            vec![frame.into_geometry(), nodes]
        }
    }

    pub struct Region {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    }

    struct Node{
        id: u32,
        bounds: Rectangle,
        color: Color,
        is_selected: bool,
    }

    impl Node {
        fn draw(&self, frame: &mut Frame, scale: f32) {
            let position = Point::new(self.bounds.x, self.bounds.y);
            let body = Path::rectangle(position, self.bounds.size());
            let normal_stroke = Stroke{
                color: self.color,
                width: 2.5 * scale,
                ..Stroke::default()
            };
            let selected_stroke = Stroke{
                color: Color::from_rgb(1., 0., 0.),
                width: 2.5 * scale,
                ..Stroke::default()
            };
            frame.fill(&body, self.color);
            if self.is_selected{
                frame.stroke(&body, selected_stroke);
            }else{
                frame.stroke(&body, normal_stroke);
            }
        }

        fn set_selected(&mut self, selected: bool){
            self.is_selected = selected;
        }

        fn set_new_pos(&mut self, new_pos: Vector){
            self.bounds.x = new_pos.x;
            self.bounds.y = new_pos.y;
        }

        fn get_pos(&self) -> Vector{
            Vector::new(self.bounds.x, self.bounds.y)
        }
    }
}

struct Panel{
    
}

impl<Message, Renderer> Widget<Message, Renderer> for Panel
    where Renderer: renderer::Renderer,
{
    fn width(&self) -> Length {
        todo!()
    }

    fn height(&self) -> Length {
        todo!()
    }

    fn layout(&self, renderer: &Renderer, limits: &Limits) -> Node {
        todo!()
    }

    fn draw(&self, renderer: &mut Renderer, style: &Style, layout: Layout<'_>, cursor_position: Point, viewport: &Rectangle) {

    }
}