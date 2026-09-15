use glam::{Mat4, Vec2, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    /// Cast ray from camera center (crosshair)
    pub fn from_camera_center(camera_eye: Vec3, camera_forward: Vec3) -> Self {
        Self::new(camera_eye, camera_forward)
    }

    /// Cast ray from arbitrary normalized viewport coordinate: x, y in [-1.0, 1.0]
    pub fn from_screen_point(
        screen_coord: Vec2,
        inv_view_proj: Mat4,
    ) -> Self {
        let near_ndc = Vec3::new(screen_coord.x, screen_coord.y, 0.0).extend(1.0);
        let far_ndc = Vec3::new(screen_coord.x, screen_coord.y, 1.0).extend(1.0);

        let near_world = inv_view_proj * near_ndc;
        let far_world = inv_view_proj * far_ndc;

        let near_world = near_world.truncate() / near_world.w;
        let far_world = far_world.truncate() / far_world.w;

        let dir = (far_world - near_world).normalize();
        Self {
            origin: near_world,
            direction: dir,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RayIntersection {
    pub distance: f32,
    pub point: Vec3,
    /// UV coordinates on quad: (0, 0) top-left, (1, 1) bottom-right
    pub uv: Vec2,
}

/// Ray - Oriented Quad intersection
/// The quad is centered at `quad_position`, rotated by `quad_rotation`, scaled by `quad_scale`.
/// Local vertices: X in [-0.5, 0.5], Y in [-0.5, 0.5], Z = 0
pub fn intersect_ray_quad(
    ray: &Ray,
    quad_transform: Mat4,
) -> Option<RayIntersection> {
    let inv_model = quad_transform.inverse();

    // Transform ray to quad's local coordinate system
    let local_origin = (inv_model * ray.origin.extend(1.0)).truncate();
    let local_dir = (inv_model * ray.direction.extend(0.0)).truncate();

    // Quad lies in local Z = 0 plane. Normal is (0, 0, 1)
    if local_dir.z.abs() < 1e-6 {
        return None; // Ray is parallel to quad plane
    }

    let t = -local_origin.z / local_dir.z;
    if t < 0.0 {
        return None; // Intersection behind ray origin
    }

    let hit_local = local_origin + local_dir * t;

    // Standard centered quad bounds: [-0.5, 0.5] for both X and Y
    if hit_local.x >= -0.5 && hit_local.x <= 0.5 && hit_local.y >= -0.5 && hit_local.y <= 0.5 {
        // Map local [-0.5, 0.5] to UV [0.0, 1.0]
        // Top-left is (-0.5, 0.5), so:
        // u = hit.x + 0.5
        // v = 0.5 - hit.y
        let u = (hit_local.x + 0.5).clamp(0.0, 1.0);
        let v = (0.5 - hit_local.y).clamp(0.0, 1.0);

        let world_point = ray.origin + ray.direction * t;

        Some(RayIntersection {
            distance: t,
            point: world_point,
            uv: Vec2::new(u, v),
        })
    } else {
        None
    }
}
