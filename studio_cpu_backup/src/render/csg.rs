#[derive(Clone)]
pub struct Cached3DFace {
    pub verts: Vec<[f32; 3]>,
    pub face_id: String,
    pub is_inner_wall: bool,
    pub part_id: String,
}

#[derive(Clone, Debug)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Clone, Debug)]
pub struct Plane {
    pub n: [f32; 3],
    pub d: f32,
}

pub fn make_plane(v0: [f32; 3], v1: [f32; 3], v2: [f32; 3]) -> Plane {
    let dx1 = v1[0] - v0[0]; let dy1 = v1[1] - v0[1]; let dz1 = v1[2] - v0[2];
    let dx2 = v2[0] - v1[0]; let dy2 = v2[1] - v1[1]; let dz2 = v2[2] - v1[2];
    
    let mut nx = dy1 * dz2 - dz1 * dy2;
    let mut ny = dz1 * dx2 - dx1 * dz2;
    let mut nz = dx1 * dy2 - dy1 * dx2;
    
    let len = (nx*nx + ny*ny + nz*nz).sqrt();
    if len > 0.00001 { nx /= len; ny /= len; nz /= len; }
    
    let d = -(nx * v0[0] + ny * v0[1] + nz * v0[2]);
    Plane { n: [nx, ny, nz], d }
}

const EPSILON: f32 = 1e-4;

pub fn split_poly(poly: &[Vertex], plane: &Plane, is_subtract_vol: bool) -> (Vec<Vertex>, Vec<Vertex>) {
    let mut outside = Vec::new();
    let mut inside = Vec::new();
    
    if poly.is_empty() { return (outside, inside); }
    
    let mut dists = Vec::with_capacity(poly.len());
    let mut types = Vec::with_capacity(poly.len()); // 1 = Front, -1 = Back, 0 = On Plane
    let mut has_front = false;
    let mut has_back = false;

    for v in poly {
        let d = plane.n[0]*v.pos[0] + plane.n[1]*v.pos[1] + plane.n[2]*v.pos[2] + plane.d;
        dists.push(d);
        if d > EPSILON {
            types.push(1);
            has_front = true;
        } else if d < -EPSILON {
            types.push(-1);
            has_back = true;
        } else {
            types.push(0);
        }
    }
    
    // Yüzeyin tamamı tek bir tarafta kalıyorsa veya tam çizginin üzerindeyse (Coplanar)
    if !has_front && !has_back {
        // Yüzey tam olarak kesme düzleminin üzerinde (Coplanar Intersection)!
        // Açı (Normal) hesabı yaparak kaymaları ve Z-fighting hatalarını önle.
        let mut dot = 0.0;
        if poly.len() >= 3 {
            let p0 = poly[0].pos; let p1 = poly[1].pos; let p2 = poly[2].pos;
            let v1 = [p1[0]-p0[0], p1[1]-p0[1], p1[2]-p0[2]];
            let v2 = [p2[0]-p0[0], p2[1]-p0[1], p2[2]-p0[2]];
            let nx = v1[1]*v2[2] - v1[2]*v2[1];
            let ny = v1[2]*v2[0] - v1[0]*v2[2];
            let nz = v1[0]*v2[1] - v1[1]*v2[0];
            dot = nx * plane.n[0] + ny * plane.n[1] + nz * plane.n[2];
        }
        
        if dot > 0.0 && is_subtract_vol {
            // Normaller aynı yöne bakıyor VE kesen obje bir DELİK (-). O zaman sil.
            return (Vec::new(), poly.to_vec());
        } else {
            // Normaller zıt yönde veya objelerin ikisi de KATI (+). Silme, üst üste çizilsin! (Z-fighting korunması)
            return (poly.to_vec(), Vec::new());
        }
    }

    if !has_front {
        // Tamamen içeride
        return (Vec::new(), poly.to_vec());
    }
    if !has_back {
        // Tamamen dışarıda
        return (poly.to_vec(), Vec::new());
    }
    
    for i in 0..poly.len() {
        let j = (i + 1) % poly.len();
        let v1 = &poly[i];
        let v2 = &poly[j];
        let t1 = types[i];
        let t2 = types[j];
        
        // Köşe noktalarını ilgili gruba ata
        if t1 == 1 {
            outside.push(v1.clone());
        } else if t1 == -1 {
            inside.push(v1.clone());
        } else {
            // Çizginin (düzlemin) tam üzerinde olan köşeler her iki bölüme de aittir.
            outside.push(v1.clone());
            inside.push(v1.clone());
        }
        
        // Eğer iki köşe farklı taraflardaysa (biri içeride, diğeri dışarıda), araya yeni bir kesişim köşesi ekle!
        if (t1 == 1 && t2 == -1) || (t1 == -1 && t2 == 1) {
            let t = dists[i] / (dists[i] - dists[j]);
            let pos = [
                v1.pos[0] + t * (v2.pos[0] - v1.pos[0]),
                v1.pos[1] + t * (v2.pos[1] - v1.pos[1]),
                v1.pos[2] + t * (v2.pos[2] - v1.pos[2]),
            ];
            let uv = [
                v1.uv[0] + t * (v2.uv[0] - v1.uv[0]),
                v1.uv[1] + t * (v2.uv[1] - v1.uv[1]),
            ];
            
            // Sonsuz Uzama (Spikes) ve NaN Koruması!
            if !pos[0].is_nan() && !pos[1].is_nan() && !pos[2].is_nan() &&
               pos[0].is_finite() && pos[1].is_finite() && pos[2].is_finite() {
                let intersect = Vertex { pos, uv };
                outside.push(intersect.clone());
                inside.push(intersect);
            }
        }
    }
    
    // Köşe Temizleyici: Kesişim algoritmasının oluşturduğu çok çok yakın (floating point error) 
    // veya kopya köşeleri birleştir. Dışbükey (convex) yapıyı korur.
    let clean_poly = |p: Vec<Vertex>| -> Vec<Vertex> {
        if p.len() < 3 { return Vec::new(); }
        let mut cleaned: Vec<Vertex> = Vec::new();
        for v in p {
            if let Some(last) = cleaned.last() {
                let dx = v.pos[0] - last.pos[0];
                let dy = v.pos[1] - last.pos[1];
                let dz = v.pos[2] - last.pos[2];
                if dx*dx + dy*dy + dz*dz > 1e-6 {
                    cleaned.push(v);
                }
            } else {
                cleaned.push(v);
            }
        }
        if cleaned.len() >= 3 {
            let first = &cleaned[0];
            let last = cleaned.last().unwrap();
            let dx = first.pos[0] - last.pos[0];
            let dy = first.pos[1] - last.pos[1];
            let dz = first.pos[2] - last.pos[2];
            if dx*dx + dy*dy + dz*dz <= 1e-6 {
                cleaned.pop();
            }
        }
        if cleaned.len() < 3 { return Vec::new(); }
        cleaned
    };
    
    (clean_poly(outside), clean_poly(inside))
}

pub fn subtract_convex(poly: &[Vertex], planes: &[Plane], is_subtract_vol: bool) -> (Vec<Vec<Vertex>>, Vec<Vertex>) {
    let mut results = Vec::new();
    let mut current_poly = poly.to_vec();
    
    for plane in planes {
        let (outside, inside) = split_poly(&current_poly, plane, is_subtract_vol);
        if !outside.is_empty() {
            results.push(outside);
        }
        if inside.is_empty() {
            break;
        }
        current_poly = inside;
    }
    (results, current_poly)
}
