//====================================================================
// Uniforms

struct Camera {
    projection: mat4x4<f32>,
    position: vec3<f32>,
}

struct Ui {
    size: vec4<f32>,
    menu_color: vec4<f32>,
    selection_color: vec4<f32>,
    selection_range_y: vec4<f32>,
}

struct Position {
    transform: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var<uniform> ui: Ui;

@group(2) @binding(0) var<uniform> position: Position;


//====================================================================

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) menu_color: vec4<f32>,
    @location(2) selection_color: vec4<f32>,
    @location(3) selection_range: vec2<f32>,
}

//====================================================================

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOut {
    var out: VertexOut;

    var vertex_pos: vec2<f32>;
    
    switch (index) {
        // 0 = Top Left
        case 0u: {
            vertex_pos = vec2<f32>(-0.5, 0.5);
            out.uv = vec2(0., 0.);
            break;
        }
        // 1 = Top Right
        case 2u: {
            vertex_pos = vec2<f32>(0.5, 0.5);
            out.uv = vec2<f32>(1., 0.);
            break;
        }
        // Bottom Left
        case 1u: {
            vertex_pos = vec2<f32>(-0.5, -0.5);
            out.uv = vec2<f32>(0., 1.);
            break;
        }
        // Bottom Right
        case 3u: {
            vertex_pos = vec2<f32>(0.5, -0.5);
            out.uv = vec2<f32>(1., 1.);
            break;
        }
        default: {}
    }

    let offset = vec2<f32>(
        ui.size.x / 2.,
        -ui.size.y / 2.5
    );
    
    vertex_pos = 
        vertex_pos 
        * ui.size.xy
        + offset;

    out.clip_position =
        camera.projection
        * position.transform
        * vec4<f32>(vertex_pos, 1., 1.);

    out.menu_color = ui.menu_color;
    out.selection_color = ui.selection_color;
    out.selection_range = ui.selection_range_y.xy;

    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    if in.uv.y > in.selection_range.x && in.uv.y < in.selection_range.y {
        return in.selection_color;
    }

    return in.menu_color;
}

//====================================================================
