use crate::constants;
use bincode::{Decode, Encode};
use sdl2::image::LoadSurface;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{Canvas, Texture, TextureAccess};
use sdl2::surface::Surface;
use sdl2::ttf::{Font, Sdl2TtfContext};
use sdl2::video::Window;

macro_rules! define_textures {
    (
        $( $name:ident => $file:expr ),+ $(,)?
    ) => {
        #[derive(Copy, Clone, Debug, Encode, Decode)]
        pub enum TextureId {
            $( $name ),+
        }

        #[allow(non_snake_case)]
        pub struct Textures {
            $( $name: Option<(Texture, (u32, u32))>, )+
        }

        impl Textures {
            pub fn new() -> Self {
                Textures {
                    $( $name: None, )+
                }
            }

            pub fn init_textures(&mut self, canvas: &mut Canvas<Window>) -> Result<(), String> {
                $( self.$name = Some(load_texture(canvas, format!("assets/textures/{}", $file).as_str())?); )+
                Ok(())
            }

            pub fn get(&self, id: TextureId) -> &(Texture, (u32, u32)) {
                match id {
                    $( TextureId::$name => self.$name.as_ref().expect(format!("Texture {} not loaded, make sure you call Textures::init_textures before Textures::get", stringify!($name)).as_str()), )+
                }
            }
        }
    }
}

fn load_texture(canvas: &mut Canvas<Window>, path: &str) -> Result<(Texture, (u32, u32)), String> {
    let surf = Surface::from_file(path).map_err(|e| format!("Failed to load {path}: {e}"))?;
    let size = surf.size();
    let tex = canvas
        .create_texture_from_surface(surf)
        .map_err(|e| format!("Failed to create texture for {path}: {e}"))?;

    Ok((tex, size))
}

define_textures! {
    Title => "client/title.png",
    TitleSmallGear => "client/title_small_gear.png",
    TitleLargeGear => "client/title_large_gear.png",

    PlayButton => "client/play_button.png",
    QuitButton => "client/quit_button.png",
    IpLabel => "client/ip_label.png",
    JoinButton => "client/join_button.png",
    IpBox => "client/ip_box.png",
    UsernameLabel => "client/username_label.png",
    UsernameBox => "client/ip_box.png",

    ComponentArmLarge => "components/arm_large.png",
    ComponentArmMedium => "components/arm_medium.png",
    ComponentArmSmall => "components/arm_small.png",
    ComponentArmTiny => "components/arm_tiny.png",
    ComponentBody => "components/body.png",
    ComponentMotor => "components/motor.png",
    ComponentPiston => "components/piston.png",
    ComponentScrew => "components/screw.png",

    IconArmLarge => "building/component_icons/arm_large.png",
    IconArmMedium => "building/component_icons/arm_medium.png",
    IconArmSmall => "building/component_icons/arm_small.png",
    IconArmTiny => "building/component_icons/arm_tiny.png",
    IconBody => "building/component_icons/body.png",
    IconMotor => "building/component_icons/motor.png",
    IconPiston => "building/component_icons/piston.png",
    IconScrew => "building/component_icons/screw.png",

    MaskArmLarge => "selected_comps/arm_large.png",
    MaskArmMedium => "selected_comps/arm_medium.png",
    MaskArmSmall => "selected_comps/arm_small.png",
    MaskArmTiny => "selected_comps/arm_tiny.png",
    MaskBody => "selected_comps/body.png",
    MaskMotor => "selected_comps/motor.png",
    MaskPiston => "selected_comps/piston.png",
    MaskScrew => "selected_comps/screw.png",

    BuildingComponentBox => "building/building_component_box.png",
    BuildingStateButton => "client/building_state_button.png",
    DoneButton => "client/done_button.png",

    StartButton => "server/start_button.png",
    NextPart => "server/next_part_button.png",
    Arrow => "server/arrow.png",

    ProgrammingAcos => "building/programming_icons/acos.png",
    ProgrammingAdd => "building/programming_icons/add.png",
    ProgrammingAnd => "building/programming_icons/and.png",
    ProgrammingAsin => "building/programming_icons/asin.png",
    ProgrammingAtan2 => "building/programming_icons/atan2.png",
    ProgrammingAtan => "building/programming_icons/atan.png",
    ProgrammingConst => "building/programming_icons/const.png",
    ProgrammingCos => "building/programming_icons/cos.png",
    ProgrammingDiv => "building/programming_icons/div.png",
    ProgrammingFalse => "building/programming_icons/false.png",
    ProgrammingGreaterThan => "building/programming_icons/greater_than.png",
    ProgrammingIf => "building/programming_icons/if.png",
    ProgrammingLessThan => "building/programming_icons/less_than.png",
    ProgrammingMul => "building/programming_icons/mul.png",
    ProgrammingNeg => "building/programming_icons/neg.png",
    ProgrammingNot => "building/programming_icons/not.png",
    ProgrammingOnKeyDown => "building/programming_icons/onkeydown.png",
    ProgrammingOnKeyUp => "building/programming_icons/onkeyup.png",
    ProgrammingOr => "building/programming_icons/or.png",
    ProgrammingPow => "building/programming_icons/pow.png",
    ProgrammingSetState => "building/programming_icons/setstate.png",
    ProgrammingSin => "building/programming_icons/sin.png",
    ProgrammingSqrt => "building/programming_icons/sqrt.png",
    ProgrammingSub => "building/programming_icons/sub.png",
    ProgrammingTan => "building/programming_icons/tan.png",
    ProgrammingTernary => "building/programming_icons/ternary.png",
    ProgrammingTrue => "building/programming_icons/true.png",
    ProgrammingXor => "building/programming_icons/xor.png",

    SelectedProgrammingAcos => "building/selected_programming_icons/acos.png",
    SelectedProgrammingAdd => "building/selected_programming_icons/add.png",
    SelectedProgrammingAnd => "building/selected_programming_icons/and.png",
    SelectedProgrammingAsin => "building/selected_programming_icons/asin.png",
    SelectedProgrammingAtan2 => "building/selected_programming_icons/atan2.png",
    SelectedProgrammingAtan => "building/selected_programming_icons/atan.png",
    SelectedProgrammingConst => "building/selected_programming_icons/const.png",
    SelectedProgrammingCos => "building/selected_programming_icons/cos.png",
    SelectedProgrammingDiv => "building/selected_programming_icons/div.png",
    SelectedProgrammingFalse => "building/selected_programming_icons/false.png",
    SelectedProgrammingGreaterThan => "building/selected_programming_icons/greater_than.png",
    SelectedProgrammingIf => "building/selected_programming_icons/if.png",
    SelectedProgrammingLessThan => "building/selected_programming_icons/less_than.png",
    SelectedProgrammingMul => "building/selected_programming_icons/mul.png",
    SelectedProgrammingNeg => "building/selected_programming_icons/neg.png",
    SelectedProgrammingNot => "building/selected_programming_icons/not.png",
    SelectedProgrammingOnKeyDown => "building/selected_programming_icons/onkeydown.png",
    SelectedProgrammingOnKeyUp => "building/selected_programming_icons/onkeyup.png",
    SelectedProgrammingOr => "building/selected_programming_icons/or.png",
    SelectedProgrammingPow => "building/selected_programming_icons/pow.png",
    SelectedProgrammingSetState => "building/selected_programming_icons/setstate.png",
    SelectedProgrammingSin => "building/selected_programming_icons/sin.png",
    SelectedProgrammingSqrt => "building/selected_programming_icons/sqrt.png",
    SelectedProgrammingSub => "building/selected_programming_icons/sub.png",
    SelectedProgrammingTan => "building/selected_programming_icons/tan.png",
    SelectedProgrammingTernary => "building/selected_programming_icons/ternary.png",
    SelectedProgrammingTrue => "building/selected_programming_icons/true.png",
    SelectedProgrammingXor => "building/selected_programming_icons/xor.png",

    PartSettingsBackground => "building/part_settings_background.png"
}

pub struct TextureHandler<'ttf> {
    font: Font<'ttf, 'ttf>,
    textures: Textures,
}

impl<'ttf> TextureHandler<'ttf> {
    pub fn new(
        ttf_context: &'ttf Sdl2TtfContext,
        canvas: &mut Canvas<Window>,
    ) -> Result<TextureHandler<'ttf>, String> {
        let font = ttf_context
            .load_font("assets/font/8bit.ttf", 32)
            .map_err(|e| e.to_string())?;

        let mut textures = Textures::new();

        if let Err(e) = textures.init_textures(canvas) {
            eprintln!("Error initializing textures: {}", e);
        }

        Ok(TextureHandler { font, textures })
    }

    pub fn render_text(
        &self,
        text: &str,
        canvas: &mut Canvas<Window>,
        color: Color,
    ) -> Result<(Texture, (u32, u32)), String> {
        if text.is_empty() {
            return Ok((
                canvas
                    .create_texture(PixelFormatEnum::RGBA8888, TextureAccess::Static, 1, 1)
                    .map_err(|e| e.to_string())?,
                (0, 0),
            ));
        }

        let surf = self
            .font
            .render(text)
            .solid(color)
            .map_err(|e| e.to_string())?;
        let size = surf.size();
        let tex = canvas
            .create_texture_from_surface(&surf)
            .map_err(|e| e.to_string())?;

        Ok((tex, size))
    }

    pub fn get_texture(&self, texture: TextureId) -> (&Texture, (u32, u32)) {
        let (tex, (w, h)) = self.textures.get(texture);

        (
            tex,
            (*w * constants::TEXTURE_SCALE, *h * constants::TEXTURE_SCALE),
        )
    }
}

pub fn destroy(tex: (Texture, (u32, u32))) {
    unsafe {
        tex.0.destroy();
    }
}
