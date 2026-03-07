//! Animation part
//!
//! Represents slide animations and transitions.
//!
//! # Animation Types
//! - Entrance animations (appear, fade, fly in, etc.)
//! - Exit animations (disappear, fade out, fly out, etc.)
//! - Emphasis animations (pulse, spin, grow/shrink, etc.)
//! - Motion path animations
//! - Slide transitions

use crate::exc::PptxError;

/// Animation effect type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationEffect {
    // Entrance effects
    Appear,
    Fade,
    FlyIn,
    Float,
    Split,
    Wipe,
    Shape,
    Wheel,
    RandomBars,
    GrowAndTurn,
    Zoom,
    Swivel,
    Bounce,
    // Exit effects
    Disappear,
    FadeOut,
    FlyOut,
    FloatOut,
    // Emphasis effects
    Pulse,
    ColorPulse,
    Teeter,
    Spin,
    GrowShrink,
    Desaturate,
    Darken,
    Lighten,
    Transparency,
    ObjectColor,
    // Motion paths
    Lines,
    Arcs,
    Turns,
    Shapes,
    Loops,
    Custom,
}

impl AnimationEffect {
    pub fn preset_id(&self) -> u32 {
        match self {
            AnimationEffect::Appear => 1,
            AnimationEffect::Fade => 10,
            AnimationEffect::FlyIn => 2,
            AnimationEffect::Float => 14,
            AnimationEffect::Split => 16,
            AnimationEffect::Wipe => 22,
            AnimationEffect::Shape => 17,
            AnimationEffect::Wheel => 21,
            AnimationEffect::RandomBars => 15,
            AnimationEffect::GrowAndTurn => 26,
            AnimationEffect::Zoom => 23,
            AnimationEffect::Swivel => 19,
            AnimationEffect::Bounce => 25,
            AnimationEffect::Disappear => 1,
            AnimationEffect::FadeOut => 10,
            AnimationEffect::FlyOut => 2,
            AnimationEffect::FloatOut => 14,
            AnimationEffect::Pulse => 31,
            AnimationEffect::ColorPulse => 32,
            AnimationEffect::Teeter => 33,
            AnimationEffect::Spin => 34,
            AnimationEffect::GrowShrink => 35,
            AnimationEffect::Desaturate => 36,
            AnimationEffect::Darken => 37,
            AnimationEffect::Lighten => 38,
            AnimationEffect::Transparency => 39,
            AnimationEffect::ObjectColor => 40,
            AnimationEffect::Lines => 42,
            AnimationEffect::Arcs => 43,
            AnimationEffect::Turns => 44,
            AnimationEffect::Shapes => 45,
            AnimationEffect::Loops => 46,
            AnimationEffect::Custom => 47,
        }
    }

    pub fn preset_class(&self) -> &'static str {
        match self {
            AnimationEffect::Appear | AnimationEffect::Fade | AnimationEffect::FlyIn |
            AnimationEffect::Float | AnimationEffect::Split | AnimationEffect::Wipe |
            AnimationEffect::Shape | AnimationEffect::Wheel | AnimationEffect::RandomBars |
            AnimationEffect::GrowAndTurn | AnimationEffect::Zoom | AnimationEffect::Swivel |
            AnimationEffect::Bounce => "entr",
            AnimationEffect::Disappear | AnimationEffect::FadeOut | AnimationEffect::FlyOut |
            AnimationEffect::FloatOut => "exit",
            AnimationEffect::Pulse | AnimationEffect::ColorPulse | AnimationEffect::Teeter |
            AnimationEffect::Spin | AnimationEffect::GrowShrink | AnimationEffect::Desaturate |
            AnimationEffect::Darken | AnimationEffect::Lighten | AnimationEffect::Transparency |
            AnimationEffect::ObjectColor => "emph",
            AnimationEffect::Lines | AnimationEffect::Arcs | AnimationEffect::Turns |
            AnimationEffect::Shapes | AnimationEffect::Loops | AnimationEffect::Custom => "path",
        }
    }
}

/// Animation trigger
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimationTrigger {
    #[default]
    OnClick,
    WithPrevious,
    AfterPrevious,
}

impl AnimationTrigger {
    pub fn as_str(&self) -> &'static str {
        match self {
            AnimationTrigger::OnClick => "onClick",
            AnimationTrigger::WithPrevious => "withPrev",
            AnimationTrigger::AfterPrevious => "afterPrev",
        }
    }
}

/// Animation direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimationDirection {
    #[default]
    In,
    Out,
    Up,
    Down,
    Left,
    Right,
    UpLeft,
    UpRight,
    DownLeft,
    DownRight,
}

impl AnimationDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            AnimationDirection::In => "in",
            AnimationDirection::Out => "out",
            AnimationDirection::Up => "u",
            AnimationDirection::Down => "d",
            AnimationDirection::Left => "l",
            AnimationDirection::Right => "r",
            AnimationDirection::UpLeft => "ul",
            AnimationDirection::UpRight => "ur",
            AnimationDirection::DownLeft => "dl",
            AnimationDirection::DownRight => "dr",
        }
    }
}

/// Single animation on a shape
#[derive(Debug, Clone)]
pub struct Animation {
    pub shape_id: u32,
    pub effect: AnimationEffect,
    pub trigger: AnimationTrigger,
    pub direction: AnimationDirection,
    pub duration_ms: u32,
    pub delay_ms: u32,
    pub repeat_count: Option<u32>,
    pub auto_reverse: bool,
}

impl Animation {
    /// Create a new animation
    pub fn new(shape_id: u32, effect: AnimationEffect) -> Self {
        Animation {
            shape_id,
            effect,
            trigger: AnimationTrigger::default(),
            direction: AnimationDirection::default(),
            duration_ms: 500,
            delay_ms: 0,
            repeat_count: None,
            auto_reverse: false,
        }
    }

    /// Set trigger
    pub fn trigger(mut self, trigger: AnimationTrigger) -> Self {
        self.trigger = trigger;
        self
    }

    /// Set direction
    pub fn direction(mut self, direction: AnimationDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Set duration in milliseconds
    pub fn duration(mut self, ms: u32) -> Self {
        self.duration_ms = ms;
        self
    }

    /// Set delay in milliseconds
    pub fn delay(mut self, ms: u32) -> Self {
        self.delay_ms = ms;
        self
    }

    /// Set repeat count (None = no repeat)
    pub fn repeat(mut self, count: u32) -> Self {
        self.repeat_count = Some(count);
        self
    }

    /// Set auto reverse
    pub fn auto_reverse(mut self) -> Self {
        self.auto_reverse = true;
        self
    }

    /// Generate animation XML
    pub fn to_xml(&self, seq_id: u32) -> String {
        let repeat_attr = self.repeat_count
            .map(|c| format!(r#" repeatCount="{}000""#, c))
            .unwrap_or_default();
        let reverse_attr = if self.auto_reverse { r#" autoRev="1""# } else { "" };

        format!(
            r#"<p:par>
  <p:cTn id="{}" presetID="{}" presetClass="{}" presetSubtype="0" fill="hold" nodeType="{}">
    <p:stCondLst>
      <p:cond delay="{}"/>
    </p:stCondLst>
    <p:childTnLst>
      <p:set>
        <p:cBhvr>
          <p:cTn id="{}" dur="{}" fill="hold"{}{}>
            <p:stCondLst><p:cond delay="0"/></p:stCondLst>
          </p:cTn>
          <p:tgtEl>
            <p:spTgt spid="{}"/>
          </p:tgtEl>
        </p:cBhvr>
      </p:set>
    </p:childTnLst>
  </p:cTn>
</p:par>"#,
            seq_id,
            self.effect.preset_id(),
            self.effect.preset_class(),
            self.trigger.as_str(),
            self.delay_ms,
            seq_id + 1,
            self.duration_ms,
            repeat_attr,
            reverse_attr,
            self.shape_id
        )
    }
}

/// Slide transition effect
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransitionEffect {
    #[default]
    None,
    Fade,
    Push,
    Wipe,
    Split,
    Reveal,
    RandomBars,
    Shape,
    Uncover,
    Cover,
    Flash,
    Strips,
    Blinds,
    Clock,
    Ripple,
    Honeycomb,
    Glitter,
    Vortex,
    Shred,
    Switch,
    Flip,
    Gallery,
    Cube,
    Doors,
    Box,
    Zoom,
    Random,
}

impl TransitionEffect {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransitionEffect::None => "none",
            TransitionEffect::Fade => "fade",
            TransitionEffect::Push => "push",
            TransitionEffect::Wipe => "wipe",
            TransitionEffect::Split => "split",
            TransitionEffect::Reveal => "reveal",
            TransitionEffect::RandomBars => "randomBar",
            TransitionEffect::Shape => "circle",
            TransitionEffect::Uncover => "pull",
            TransitionEffect::Cover => "cover",
            TransitionEffect::Flash => "flash",
            TransitionEffect::Strips => "strips",
            TransitionEffect::Blinds => "blinds",
            TransitionEffect::Clock => "wheel",
            TransitionEffect::Ripple => "ripple",
            TransitionEffect::Honeycomb => "honeycomb",
            TransitionEffect::Glitter => "glitter",
            TransitionEffect::Vortex => "vortex",
            TransitionEffect::Shred => "shred",
            TransitionEffect::Switch => "switch",
            TransitionEffect::Flip => "flip",
            TransitionEffect::Gallery => "gallery",
            TransitionEffect::Cube => "cube",
            TransitionEffect::Doors => "doors",
            TransitionEffect::Box => "box",
            TransitionEffect::Zoom => "zoom",
            TransitionEffect::Random => "random",
        }
    }
}

/// Slide transition
#[derive(Debug, Clone)]
pub struct SlideTransition {
    pub effect: TransitionEffect,
    pub duration_ms: u32,
    pub direction: AnimationDirection,
    pub advance_on_click: bool,
    pub advance_after_ms: Option<u32>,
}

impl Default for SlideTransition {
    fn default() -> Self {
        SlideTransition {
            effect: TransitionEffect::None,
            duration_ms: 500,
            direction: AnimationDirection::default(),
            advance_on_click: true,
            advance_after_ms: None,
        }
    }
}

impl SlideTransition {
    pub fn new(effect: TransitionEffect) -> Self {
        SlideTransition {
            effect,
            ..Default::default()
        }
    }

    pub fn duration(mut self, ms: u32) -> Self {
        self.duration_ms = ms;
        self
    }

    pub fn direction(mut self, dir: AnimationDirection) -> Self {
        self.direction = dir;
        self
    }

    pub fn advance_after(mut self, ms: u32) -> Self {
        self.advance_after_ms = Some(ms);
        self
    }

    pub fn no_click_advance(mut self) -> Self {
        self.advance_on_click = false;
        self
    }

    pub fn to_xml(&self) -> String {
        if self.effect == TransitionEffect::None {
            return String::new();
        }

        let advance_attr = if self.advance_on_click { "" } else { r#" advClick="0""# };
        let auto_advance = self.advance_after_ms
            .map(|ms| format!(r#" advTm="{}""#, ms))
            .unwrap_or_default();

        format!(
            r#"<p:transition spd="med"{}{}>
  <p:{} dir="{}"/>
</p:transition>"#,
            advance_attr,
            auto_advance,
            self.effect.as_str(),
            self.direction.as_str()
        )
    }
}

/// Animation sequence for a slide
#[derive(Debug, Clone, Default)]
pub struct SlideAnimations {
    pub animations: Vec<Animation>,
    pub transition: Option<SlideTransition>,
}

impl SlideAnimations {
    pub fn new() -> Self {
        SlideAnimations::default()
    }

    /// Add an animation
    pub fn add(mut self, animation: Animation) -> Self {
        self.animations.push(animation);
        self
    }

    /// Set slide transition
    pub fn transition(mut self, transition: SlideTransition) -> Self {
        self.transition = Some(transition);
        self
    }

    /// Generate timing XML for slide
    pub fn to_timing_xml(&self) -> Result<String, PptxError> {
        if self.animations.is_empty() {
            return Ok(String::new());
        }

        let animations_xml: String = self.animations.iter()
            .enumerate()
            .map(|(i, a)| a.to_xml((i * 2 + 1) as u32))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(format!(
            r#"<p:timing>
  <p:tnLst>
    <p:par>
      <p:cTn id="1" dur="indefinite" restart="never" nodeType="tmRoot">
        <p:childTnLst>
          <p:seq concurrent="1" nextAc="seek">
            <p:cTn id="2" dur="indefinite" nodeType="mainSeq">
              <p:childTnLst>
                {}
              </p:childTnLst>
            </p:cTn>
          </p:seq>
        </p:childTnLst>
      </p:cTn>
    </p:par>
  </p:tnLst>
</p:timing>"#,
            animations_xml
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_new() {
        let anim = Animation::new(2, AnimationEffect::Fade);
        assert_eq!(anim.shape_id, 2);
        assert_eq!(anim.duration_ms, 500);
    }

    #[test]
    fn test_animation_builder() {
        let anim = Animation::new(3, AnimationEffect::FlyIn)
            .trigger(AnimationTrigger::WithPrevious)
            .direction(AnimationDirection::Left)
            .duration(1000)
            .delay(500);
        assert_eq!(anim.duration_ms, 1000);
        assert_eq!(anim.delay_ms, 500);
    }

    #[test]
    fn test_animation_to_xml() {
        let anim = Animation::new(2, AnimationEffect::Fade);
        let xml = anim.to_xml(1);
        assert!(xml.contains("p:par"));
        assert!(xml.contains("spid=\"2\""));
    }

    #[test]
    fn test_transition_new() {
        let trans = SlideTransition::new(TransitionEffect::Fade);
        assert_eq!(trans.effect, TransitionEffect::Fade);
        assert!(trans.advance_on_click);
    }

    #[test]
    fn test_transition_to_xml() {
        let trans = SlideTransition::new(TransitionEffect::Wipe)
            .direction(AnimationDirection::Left)
            .duration(1000);
        let xml = trans.to_xml();
        assert!(xml.contains("p:transition"));
        assert!(xml.contains("p:wipe"));
    }

    #[test]
    fn test_slide_animations() {
        let anims = SlideAnimations::new()
            .add(Animation::new(2, AnimationEffect::Fade))
            .add(Animation::new(3, AnimationEffect::FlyIn))
            .transition(SlideTransition::new(TransitionEffect::Push));
        assert_eq!(anims.animations.len(), 2);
        assert!(anims.transition.is_some());
    }

    #[test]
    fn test_slide_animations_to_xml() {
        let anims = SlideAnimations::new()
            .add(Animation::new(2, AnimationEffect::Fade));
        let xml = anims.to_timing_xml().unwrap();
        assert!(xml.contains("p:timing"));
        assert!(xml.contains("p:tnLst"));
    }

    #[test]
    fn test_effect_preset_class() {
        assert_eq!(AnimationEffect::Fade.preset_class(), "entr");
        assert_eq!(AnimationEffect::FadeOut.preset_class(), "exit");
        assert_eq!(AnimationEffect::Pulse.preset_class(), "emph");
        assert_eq!(AnimationEffect::Lines.preset_class(), "path");
    }
}
