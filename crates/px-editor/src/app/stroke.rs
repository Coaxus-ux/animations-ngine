use px_core::Paint;
use px_engine::{Cel, Geometry};
use px_format::edit::NodePath;
use px_ui::Input;

use super::intent::Button;
use super::{App, WindowId};
use crate::app::Status;
use crate::domain::{Command, Raster};
use crate::ports::FileStore;
use crate::tools::{Effect, Tool, ToolKind, Work};

pub(super) struct Stroke {
    window: WindowId,
    target: Option<PaintTarget>,
    base: Raster,
    raster: Raster,
    tool: Box<dyn Tool>,
    paint: Option<Paint>,
}

impl Stroke {
    pub(super) fn guide(&self) -> &[(i32, i32)] {
        self.tool.guide()
    }
}

pub(super) struct PaintTarget {
    shape: usize,
    art: NodePath,
    origin: (i32, i32),
    frame: u32,
    cels: Vec<Cel>,
    pub(super) raster: Raster,
}

impl PaintTarget {
    fn cels_with(&self, raster: &Raster) -> Vec<Cel> {
        let mut cels = self.cels.clone();
        let edited = raster.to_cel(self.frame, self.origin);
        if let Some(index) = cels.iter().position(|cel| cel.frame == self.frame) {
            cels[index] = edited;
        } else {
            let at = cels.partition_point(|cel| cel.frame < self.frame);
            cels.insert(at, edited);
        }
        cels
    }

    pub(super) const fn starting_at(mut self, frame: u32) -> Self {
        self.frame = frame;
        self
    }

    pub(super) fn has_cel(&self) -> bool {
        self.cels.iter().any(|cel| cel.frame == self.frame)
    }

    pub(super) fn without_cel(&self) -> Command {
        Command::SetCels {
            art: self.art.clone(),
            cels: self
                .cels
                .iter()
                .filter(|cel| cel.frame != self.frame)
                .cloned()
                .collect(),
        }
    }

    pub(super) fn command(&self, raster: &Raster) -> Command {
        Command::SetCels {
            art: self.art.clone(),
            cels: self.cels_with(raster),
        }
    }
}

impl App {
    pub(super) fn paint_target(&self, window: WindowId) -> Result<PaintTarget, String> {
        let document = self.document().ok_or("No document is open")?;
        let scene = document.scene().ok_or("The scene doesn't load")?;
        let view = &self.window(window).ok_or("No such window")?.view;
        let layer = view
            .layer
            .and_then(|index| document.layers().get(index))
            .ok_or("Select a pixel layer to paint on")?;
        if self.editing.locked.contains(&layer.name) {
            return Err("That layer is locked".to_owned());
        }
        let art = layer
            .art
            .as_ref()
            .filter(|_| layer.paintable())
            .ok_or("Select a pixel layer to paint on")?;
        let origin = art
            .origin
            .ok_or("This layer moves, so it can't be painted")?;
        let Some(Geometry::Pixels(pixels)) =
            scene.shapes().get(art.shape).map(|shape| &shape.geometry)
        else {
            return Err("Select a pixel layer to paint on".to_owned());
        };
        let showing = pixels.cel_at(view.frame);
        let size = scene.size();
        Ok(PaintTarget {
            shape: art.shape,
            art: art.path.clone(),
            origin,
            frame: showing.map_or(view.frame, |cel| cel.frame),
            cels: pixels.cels.clone(),
            raster: Raster::from_cel(
                showing,
                origin,
                i32::from(size.width()),
                i32::from(size.height()),
            ),
        })
    }

    pub(super) fn begin_stroke(
        &mut self,
        window: WindowId,
        at: (i32, i32),
        button: Button,
        input: &Input,
        store: &mut dyn FileStore,
    ) {
        let Some(scene) = self.document().and_then(crate::domain::Document::scene) else {
            return;
        };
        let size = scene.size();
        let kind = self.editing.tool;
        let target = match self.paint_target(window) {
            Ok(target) => Some(target),
            Err(reason) if kind.paints() => {
                self.editing.status = Status::problem(reason);
                return;
            }
            Err(_) => None,
        };
        let base = target.as_ref().map_or_else(
            || Raster::new(i32::from(size.width()), i32::from(size.height())),
            |target| target.raster.clone(),
        );
        let paint = match (kind, button) {
            (ToolKind::Eraser, _) => None,
            (_, Button::Primary) => self.editing.primary,
            (_, Button::Secondary) => self.editing.secondary,
        };
        let mut stroke = Stroke {
            window,
            target,
            raster: base.clone(),
            base,
            tool: kind.make(),
            paint,
        };
        let effect = stroke.tool.press(
            &mut work(
                &mut stroke.raster,
                &stroke.base,
                stroke.paint,
                &self.editing,
                input,
            ),
            at,
        );
        self.stroke = Some(stroke);
        self.handle(effect, at, store);
    }

    pub(super) fn drag_stroke(&mut self, at: (i32, i32), input: &Input, store: &mut dyn FileStore) {
        let Some(mut stroke) = self.stroke.take() else {
            return;
        };
        let effect = stroke.tool.drag(
            &mut work(
                &mut stroke.raster,
                &stroke.base,
                stroke.paint,
                &self.editing,
                input,
            ),
            at,
        );
        self.stroke = Some(stroke);
        self.handle(effect, at, store);
    }

    pub(super) fn end_stroke(&mut self, at: (i32, i32), input: &Input, store: &mut dyn FileStore) {
        let Some(mut stroke) = self.stroke.take() else {
            return;
        };
        let effect = stroke.tool.release(
            &mut work(
                &mut stroke.raster,
                &stroke.base,
                stroke.paint,
                &self.editing,
                input,
            ),
            at,
        );
        self.stroke = Some(stroke);
        self.handle(effect, at, store);
        self.stroke = None;
    }

    fn handle(&mut self, effect: Effect, at: (i32, i32), store: &mut dyn FileStore) {
        let Some(stroke) = &self.stroke else { return };
        let window = stroke.window;
        if let Some(selection) = effect.selection {
            self.editing.selection = selection;
        }
        if let Some(picked) = effect.picked_paint {
            let shown = || {
                self.caches
                    .get(&window)
                    .and_then(|cache| cache.paint_at(at.0, at.1))
            };
            if let Some(paint) = picked.or_else(shown) {
                self.editing.primary = Some(paint);
            }
        }
        if let Some((x, y)) = effect.picked_object {
            self.editing.picked = self
                .caches
                .get(&window)
                .and_then(|cache| cache.shape_at(x, y));
            self.editing.status = Status::info(match self.editing.picked {
                Some(shape) => format!("Picked shape #{shape}: click a swatch to recolor it"),
                None => "Nothing there to pick".to_owned(),
            });
        }
        let Some(target) = &stroke.target else { return };
        let commit = effect.commit.then(|| target.command(&stroke.raster));
        let preview = (!effect.commit && effect.repaint)
            .then(|| (target.shape, target.cels_with(&stroke.raster)));
        let Some(document) = self.document_mut() else {
            return;
        };
        if let Some(command) = commit {
            if let Err(error) = document.apply(&command, store) {
                document.cancel_preview();
                self.editing.status = Status::problem(error);
            }
        } else if let Some((shape, cels)) = preview {
            document.preview_cels(shape, cels);
        }
    }
}

fn work<'w>(
    raster: &'w mut Raster,
    base: &'w Raster,
    paint: Option<Paint>,
    editing: &'w super::Editing,
    input: &Input,
) -> Work<'w> {
    Work {
        raster,
        base,
        selection: &editing.selection,
        paint,
        filled: editing.filled,
        constrain: input.modifiers.shift,
        global: editing.global_fill || input.modifiers.shift,
    }
}
