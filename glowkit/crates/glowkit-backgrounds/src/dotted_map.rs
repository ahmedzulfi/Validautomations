//! Dotted Map: a world map drawn as a grid of dots, with animated arcs flying between cities.

use crate::{background, h01, light, Base};
use egui::epaint::{Mesh, Shape};
use egui::{pos2, vec2, Color32, Rect, Ui};
use glowkit_core::{
    ease,
    paint::{glow, gradient_polyline, quad_bezier},
    Easing,
};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------------------------
// Land mask: hand-authored, deliberately coarse continent outlines as (longitude, latitude)
// polygons. At dot-map resolution (2-3 degrees per dot) they read as the familiar world.
// ---------------------------------------------------------------------------------------------

type Poly = &'static [(f32, f32)];

const NORTH_AMERICA: Poly = &[
    (-166.0, 68.0), (-156.0, 71.3), (-140.0, 69.6), (-125.0, 70.0), (-110.0, 68.0), (-96.0, 68.5),
    (-85.0, 69.5), (-82.0, 66.5), (-88.0, 64.0), (-93.0, 60.0), (-94.0, 57.0), (-88.0, 56.0),
    (-82.0, 53.0), (-79.0, 55.0), (-77.0, 60.0), (-78.0, 62.5), (-72.0, 62.0), (-65.0, 60.0),
    (-62.0, 57.0), (-56.0, 52.0), (-60.0, 47.5), (-64.0, 45.0), (-67.0, 44.5), (-70.0, 42.0),
    (-74.0, 40.5), (-76.0, 37.0), (-76.0, 35.0), (-81.0, 31.5), (-80.0, 26.5), (-81.0, 25.2),
    (-82.7, 28.5), (-85.0, 29.8), (-89.0, 30.2), (-94.0, 29.5), (-97.3, 27.5), (-97.6, 22.5),
    (-96.0, 19.0), (-91.5, 18.5), (-90.5, 21.0), (-87.0, 21.5), (-88.0, 16.0), (-84.0, 15.5),
    (-83.5, 11.0), (-80.0, 9.0), (-77.5, 8.0), (-78.5, 9.5), (-82.0, 8.5), (-86.0, 12.0),
    (-92.0, 14.5), (-96.0, 15.8), (-105.0, 20.0), (-106.5, 23.5), (-110.0, 24.0), (-112.0, 27.0),
    (-114.8, 31.5), (-117.0, 32.5), (-120.5, 34.5), (-124.0, 40.0), (-124.5, 46.0), (-123.0, 49.0),
    (-128.0, 51.0), (-133.0, 56.0), (-138.0, 59.0), (-147.0, 60.8), (-152.0, 59.0), (-158.0, 57.0),
    (-164.0, 55.0), (-158.0, 58.5), (-162.0, 60.0), (-165.0, 62.0), (-166.0, 64.5),
];
const ARCTIC_ISLANDS: Poly = &[
    (-124.0, 74.0), (-118.0, 76.5), (-105.0, 78.5), (-92.0, 81.0), (-70.0, 82.5), (-62.0, 81.5),
    (-75.0, 78.0), (-80.0, 73.5), (-72.0, 72.0), (-67.0, 69.0), (-62.0, 66.5), (-66.0, 63.0),
    (-72.0, 64.0), (-78.0, 64.5), (-74.0, 67.0), (-82.0, 69.5), (-90.0, 71.0), (-100.0, 71.5),
    (-110.0, 71.0), (-120.0, 71.5),
];
const GREENLAND: Poly = &[
    (-73.0, 78.0), (-60.0, 82.0), (-35.0, 83.5), (-20.0, 82.0), (-18.0, 77.0), (-21.0, 72.0),
    (-24.0, 70.0), (-32.0, 68.0), (-40.0, 65.0), (-43.0, 60.0), (-48.0, 61.0), (-52.0, 65.0),
    (-54.0, 69.0), (-58.0, 75.0), (-68.0, 77.0),
];
const SOUTH_AMERICA: Poly = &[
    (-77.5, 8.0), (-72.0, 12.0), (-63.0, 10.5), (-60.0, 8.5), (-52.0, 5.0), (-50.0, 0.0),
    (-44.0, -2.5), (-35.0, -5.0), (-35.0, -9.0), (-39.0, -14.0), (-39.0, -18.0), (-41.0, -22.5),
    (-48.0, -26.0), (-48.5, -28.5), (-53.0, -33.5), (-57.0, -36.0), (-57.0, -38.5), (-62.0, -39.0),
    (-65.0, -41.0), (-63.0, -42.5), (-67.0, -46.0), (-66.0, -48.0), (-69.0, -51.0), (-68.5, -53.0),
    (-71.0, -54.5), (-74.0, -52.0), (-75.0, -47.0), (-74.0, -42.0), (-73.5, -37.0), (-71.5, -30.0),
    (-70.5, -24.0), (-70.0, -18.0), (-75.0, -15.0), (-77.0, -12.0), (-80.0, -7.0), (-81.0, -4.5),
    (-80.0, -1.0), (-79.0, 1.5), (-77.5, 4.0),
];
const EURASIA: Poly = &[
    // Iberia, along the Mediterranean, the Levant and Arabia
    (-9.0, 43.5), (-9.5, 39.0), (-8.8, 37.0), (-6.0, 36.2), (-2.0, 36.8), (0.0, 38.7), (3.0, 42.5),
    (6.5, 43.1), (8.8, 44.4), (12.5, 41.5), (16.0, 38.0), (16.5, 39.5), (18.5, 40.2), (16.0, 41.5),
    (13.5, 44.0), (12.3, 45.3), (13.7, 45.6), (15.5, 44.0), (19.5, 41.8), (19.5, 40.0), (21.0, 38.5),
    (22.5, 36.5), (23.5, 38.0), (22.8, 40.5), (26.0, 40.8), (26.5, 40.0), (27.0, 37.0), (30.0, 36.2),
    (36.0, 36.5), (35.9, 35.0), (35.0, 33.0), (34.5, 31.5), (34.9, 29.5), (35.0, 28.0), (37.0, 25.0),
    (39.0, 22.0), (41.0, 19.0), (42.5, 16.5), (43.3, 13.0), (45.0, 13.0), (52.0, 16.0), (55.5, 17.5),
    (57.8, 19.0), (59.8, 22.5), (56.4, 24.5), (56.0, 26.3), (54.0, 24.0), (51.5, 24.5), (50.8, 26.0),
    (48.0, 29.5), (50.0, 30.0), (54.0, 27.0), (57.0, 25.5), (61.5, 25.2), (66.5, 25.4),
    // India, south-east Asia, China
    (68.0, 23.7), (70.0, 21.0), (72.8, 19.0), (73.5, 15.5), (75.0, 12.0), (76.5, 8.5), (77.5, 8.0),
    (79.5, 10.5), (80.2, 13.5), (80.0, 15.8), (82.3, 17.0), (86.0, 20.0), (87.0, 21.5), (89.0, 22.0),
    (91.5, 22.5), (92.3, 20.7), (94.3, 18.5), (94.5, 16.0), (97.5, 16.5), (98.0, 13.0), (98.5, 9.0),
    (100.3, 6.5), (101.3, 3.0), (103.5, 1.3), (104.2, 1.5), (103.4, 4.5), (102.4, 6.0), (100.3, 8.5),
    (100.0, 12.5), (100.9, 13.5), (102.5, 12.0), (104.8, 8.6), (105.0, 10.5), (106.8, 10.4),
    (109.0, 11.8), (109.3, 13.5), (108.3, 16.0), (106.6, 17.5), (105.6, 19.0), (106.7, 20.7),
    (108.0, 21.5), (110.0, 21.0), (111.0, 21.5), (113.5, 22.3), (116.5, 23.0), (119.5, 25.5),
    (121.5, 28.5), (121.8, 31.0), (120.5, 33.0), (119.2, 35.0), (120.7, 36.5), (122.4, 37.0),
    (118.8, 38.0), (117.6, 39.0), (121.0, 40.8), (122.2, 40.5), (121.2, 39.0), (124.4, 40.0),
    // Korea, the Russian far east
    (126.7, 37.7), (126.4, 34.5), (129.3, 35.2), (129.4, 37.0), (128.0, 39.0), (129.8, 41.0),
    (130.7, 42.5), (133.0, 43.0), (135.5, 43.8), (138.0, 46.5), (140.3, 48.3), (140.5, 51.5),
    (141.5, 53.0), (137.5, 54.0), (135.3, 54.7), (137.0, 56.5), (142.0, 59.0), (148.0, 59.3),
    (152.0, 59.0), (155.0, 59.3), (156.5, 57.5), (156.0, 51.0), (158.3, 52.0), (160.0, 54.0),
    (162.0, 56.0), (163.3, 58.0), (162.0, 58.0), (163.5, 60.0), (165.0, 60.3), (170.0, 60.0),
    (173.0, 61.6), (178.0, 62.5), (180.0, 65.0), (180.0, 69.0),
    // the Arctic coast going west
    (170.0, 70.0), (160.0, 69.7), (150.0, 71.5), (141.0, 72.7), (130.0, 71.0), (128.0, 72.5),
    (113.0, 73.7), (108.0, 76.7), (104.0, 77.7), (98.0, 76.0), (88.0, 75.5), (80.0, 73.0),
    (72.0, 72.8), (68.0, 72.5), (66.0, 70.0), (60.0, 68.5), (54.0, 68.5), (44.0, 68.5),
    (44.0, 66.0), (40.0, 64.5), (36.0, 64.5), (33.0, 63.0), (30.0, 60.0),
    // the Baltic and the North Sea coast back to Iberia
    (28.0, 59.5), (23.5, 59.2), (24.3, 58.4), (23.5, 57.2), (21.0, 57.0), (21.0, 55.3),
    (18.5, 54.6), (14.3, 53.9), (12.5, 54.4), (10.5, 55.0), (10.5, 57.7), (8.2, 57.0), (8.6, 55.5),
    (8.5, 53.7), (5.0, 53.4), (4.0, 51.5), (1.5, 50.0), (-1.5, 48.7), (-4.7, 48.4), (-2.5, 47.3),
    (-1.2, 46.0), (-1.5, 43.4),
];
const SCANDINAVIA: Poly = &[
    (5.0, 62.0), (5.0, 59.0), (7.0, 58.0), (10.5, 59.0), (11.5, 58.0), (12.7, 56.0), (14.2, 55.4),
    (16.0, 56.3), (16.5, 57.5), (18.5, 59.5), (17.5, 61.0), (17.5, 62.5), (21.0, 64.5), (22.5, 65.8),
    (25.0, 65.4), (25.5, 64.3), (21.5, 62.5), (21.5, 60.5), (23.0, 60.0), (28.0, 60.5), (31.0, 62.0),
    (33.0, 66.0), (41.0, 67.0), (40.0, 69.3), (33.0, 69.5), (28.0, 71.0), (23.0, 70.7), (16.0, 69.0),
    (12.0, 66.0), (10.0, 63.5),
];
const AFRICA: Poly = &[
    (-17.0, 21.0), (-16.5, 24.0), (-13.0, 27.5), (-9.8, 29.6), (-9.5, 32.5), (-6.8, 34.0),
    (-5.9, 35.8), (-2.0, 35.2), (3.0, 36.8), (10.0, 37.3), (11.0, 35.0), (10.2, 34.0), (11.5, 33.0),
    (15.0, 32.3), (19.8, 30.5), (20.2, 32.3), (23.0, 32.6), (25.0, 31.7), (29.0, 30.9), (32.3, 31.3),
    (34.2, 31.3), (34.9, 29.5), (33.8, 27.5), (35.5, 24.0), (37.2, 21.0), (38.5, 18.0), (39.7, 15.3),
    (41.5, 13.5), (43.4, 11.8), (45.0, 10.5), (51.0, 11.8), (51.2, 10.4), (50.0, 8.0), (48.0, 5.0),
    (46.0, 2.0), (43.0, -1.0), (40.5, -2.5), (39.3, -5.0), (39.0, -7.0), (39.5, -10.5), (40.5, -11.0),
    (40.5, -15.0), (37.0, -17.5), (35.3, -22.0), (35.5, -24.0), (32.8, -26.0), (32.5, -28.7),
    (30.0, -31.3), (27.5, -33.3), (25.5, -34.0), (22.0, -34.2), (20.0, -34.8), (18.4, -34.0),
    (18.0, -31.5), (16.5, -28.5), (15.0, -26.5), (14.5, -22.5), (13.0, -20.0), (11.8, -17.5),
    (12.0, -13.5), (13.7, -11.0), (12.2, -6.0), (12.0, -5.0), (9.0, -1.0), (9.5, 1.0), (9.8, 3.5),
    (8.5, 4.5), (6.0, 4.3), (4.5, 6.3), (1.5, 6.1), (-2.0, 4.8), (-4.5, 5.2), (-7.5, 4.4), (-9.5, 5.5),
    (-11.5, 7.0), (-13.0, 8.5), (-15.0, 10.8), (-16.8, 12.4), (-17.5, 14.7), (-16.5, 16.2),
    (-16.0, 18.5),
];
const MADAGASCAR: Poly =
    &[(49.3, -12.0), (50.5, -15.5), (49.5, -17.5), (48.0, -22.0), (47.0, -25.0), (45.0, -25.5), (43.5, -22.0), (44.0, -17.0), (46.5, -15.5)];
const AUSTRALIA: Poly = &[
    (113.5, -22.0), (114.0, -26.0), (115.0, -30.0), (115.0, -34.0), (117.5, -35.0), (120.0, -34.0),
    (123.5, -34.0), (126.0, -32.3), (129.0, -31.6), (131.5, -31.5), (134.0, -32.8), (136.0, -34.8),
    (138.0, -35.5), (138.5, -34.0), (140.0, -37.5), (143.5, -38.8), (146.5, -39.0), (150.0, -37.5),
    (151.0, -34.0), (153.5, -28.5), (153.0, -25.0), (150.8, -22.5), (149.0, -20.3), (146.2, -18.5),
    (145.5, -15.0), (143.6, -14.0), (142.5, -10.7), (141.5, -13.0), (141.6, -16.0), (140.7, -17.5),
    (139.0, -17.0), (136.5, -15.5), (136.8, -12.2), (132.5, -11.5), (130.0, -13.0), (129.5, -15.0),
    (127.0, -14.0), (125.0, -15.0), (122.3, -17.0), (121.0, -19.5), (117.0, -20.8),
];
const TASMANIA: Poly = &[(144.5, -40.7), (148.3, -40.9), (148.0, -43.2), (146.0, -43.5)];
const NZ_NORTH: Poly = &[(172.7, -34.4), (175.5, -37.0), (178.5, -37.7), (176.9, -39.5), (175.0, -41.5), (174.6, -39.3)];
const NZ_SOUTH: Poly =
    &[(172.7, -40.5), (174.3, -41.7), (173.0, -43.8), (171.2, -44.5), (169.0, -46.6), (166.5, -46.0), (168.3, -44.0), (171.0, -42.3)];
const JAPAN: Poly = &[
    (129.5, 33.2), (131.0, 31.0), (132.0, 33.8), (135.0, 33.5), (137.0, 34.5), (140.0, 35.0),
    (140.9, 37.0), (141.8, 40.5), (141.4, 41.4), (139.9, 40.5), (139.5, 38.5), (137.0, 37.0),
    (135.5, 35.5), (132.5, 35.4),
];
const HOKKAIDO: Poly =
    &[(140.0, 41.5), (141.5, 42.5), (143.3, 42.0), (145.5, 43.3), (144.3, 44.2), (141.7, 45.4), (141.3, 43.3), (140.0, 42.7)];
const SAKHALIN: Poly = &[(142.0, 46.0), (143.5, 49.5), (143.0, 54.0), (142.5, 54.3), (142.0, 51.0)];
const GREAT_BRITAIN: Poly = &[
    (-5.7, 50.0), (1.5, 51.0), (1.7, 52.7), (0.0, 53.5), (-1.5, 55.0), (-2.0, 57.0), (-1.8, 57.6),
    (-3.3, 58.6), (-5.0, 58.6), (-6.2, 57.5), (-5.6, 56.0), (-4.9, 54.8), (-3.2, 54.2), (-3.0, 53.4),
    (-4.5, 52.8), (-5.2, 51.7), (-3.5, 51.4),
];
const IRELAND: Poly = &[(-6.0, 52.0), (-6.0, 54.5), (-7.5, 55.3), (-10.0, 54.2), (-10.3, 51.8), (-8.0, 51.6)];
const ICELAND: Poly = &[(-24.0, 65.5), (-22.0, 66.4), (-16.5, 66.5), (-14.0, 65.5), (-14.5, 64.4), (-18.7, 63.4), (-22.7, 63.8)];
const SUMATRA: Poly = &[
    (95.3, 5.5), (97.5, 5.2), (100.5, 2.0), (104.5, -1.5), (106.0, -3.0), (105.8, -5.8), (104.5, -5.9),
    (102.3, -4.0), (100.5, -1.5), (98.7, 1.7),
];
const JAVA: Poly = &[
    (105.2, -6.8), (106.4, -6.0), (108.3, -6.3), (111.0, -6.4), (112.6, -6.9), (114.5, -7.7),
    (114.4, -8.7), (111.0, -8.3), (108.0, -7.8), (106.0, -7.4),
];
const BORNEO: Poly = &[
    (109.0, 1.5), (109.6, -1.0), (110.2, -2.9), (114.5, -3.7), (116.5, -3.0), (118.0, 0.8), (119.0, 5.0),
    (117.2, 7.0), (115.5, 5.2), (113.0, 3.2), (111.0, 1.6),
];
const SULAWESI: Poly = &[(119.5, -5.5), (120.5, -2.0), (121.0, 1.0), (124.5, 1.5), (122.5, -1.0), (121.5, -4.5)];
const NEW_GUINEA: Poly = &[
    (131.0, -1.0), (134.0, -0.8), (137.5, -1.5), (141.0, -2.6), (145.5, -4.5), (147.5, -6.5),
    (150.0, -10.5), (147.0, -10.0), (144.0, -7.8), (141.0, -9.2), (138.3, -8.3), (137.8, -5.5),
    (134.0, -4.0), (132.0, -2.8),
];
const LUZON: Poly = &[(120.0, 18.5), (122.3, 18.5), (122.0, 16.0), (124.0, 13.0), (121.5, 13.7), (120.5, 14.5), (119.8, 16.3)];
const MINDANAO: Poly = &[(122.0, 7.0), (125.5, 9.7), (126.6, 7.3), (125.4, 5.6), (123.6, 7.8)];
const TAIWAN: Poly = &[(120.3, 22.6), (121.0, 25.2), (121.9, 24.9), (120.9, 22.0)];
const SRI_LANKA: Poly = &[(79.8, 6.2), (80.0, 9.7), (81.9, 7.5), (81.5, 6.2)];
const CUBA: Poly = &[(-84.9, 21.9), (-82.0, 23.1), (-77.5, 21.8), (-74.2, 20.3), (-77.7, 19.9), (-80.5, 21.8)];
const HISPANIOLA: Poly = &[(-74.4, 18.4), (-72.8, 19.9), (-69.9, 19.6), (-68.4, 18.6), (-71.4, 17.7)];

const LAND: &[Poly] = &[
    NORTH_AMERICA, ARCTIC_ISLANDS, GREENLAND, SOUTH_AMERICA, EURASIA, SCANDINAVIA, AFRICA, MADAGASCAR,
    AUSTRALIA, TASMANIA, NZ_NORTH, NZ_SOUTH, JAPAN, HOKKAIDO, SAKHALIN, GREAT_BRITAIN, IRELAND, ICELAND,
    SUMATRA, JAVA, BORNEO, SULAWESI, NEW_GUINEA, LUZON, MINDANAO, TAIWAN, SRI_LANKA, CUBA, HISPANIOLA,
];

/// Inland seas cut back out of the land.
const WATER: &[Poly] = &[
    // Black Sea
    &[(28.0, 41.5), (28.5, 43.5), (30.0, 45.5), (33.0, 46.0), (36.5, 45.3), (39.0, 47.2), (41.5, 42.0), (36.0, 41.7), (32.0, 41.8)],
    // Caspian Sea
    &[
        (47.0, 44.5), (49.0, 46.5), (52.0, 46.8), (53.5, 45.0), (51.0, 44.0), (52.5, 42.0), (54.0, 40.0),
        (53.5, 37.5), (50.5, 37.0), (49.0, 38.5), (49.5, 40.5), (47.5, 42.0),
    ],
];

fn inside(poly: Poly, x: f32, y: f32) -> bool {
    let mut c = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            c = !c;
        }
        j = i;
    }
    c
}

fn bbox(poly: Poly) -> [f32; 4] {
    let mut b = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for &(x, y) in poly {
        b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
    }
    b
}

/// Whether the point (longitude, latitude in degrees) is on land, according to glowkit's
/// built-in coarse world outline.
pub fn is_land(lon: f32, lat: f32) -> bool {
    let test = |polys: &[Poly]| {
        polys.iter().any(|p| {
            let b = bbox(p);
            lon >= b[0] && lon <= b[2] && lat >= b[1] && lat <= b[3] && inside(p, lon, lat)
        })
    };
    test(LAND) && !test(WATER)
}

/// Land mask at 1 degree resolution: index `(89 - lat) * 360 + (lon + 180)`.
fn mask() -> &'static [bool] {
    static MASK: OnceLock<Vec<bool>> = OnceLock::new();
    MASK.get_or_init(|| {
        let mut m = vec![false; 360 * 180];
        for row in 0..180 {
            let lat = 89.5 - row as f32;
            for col in 0..360 {
                let lon = -179.5 + col as f32;
                m[row * 360 + col] = is_land(lon, lat);
            }
        }
        m
    })
}

fn land_at(lon: f32, lat: f32) -> bool {
    let col = ((lon + 180.0).floor() as i32).clamp(0, 359) as usize;
    let row = ((90.0 - lat).floor() as i32).clamp(0, 179) as usize;
    mask()[row * 360 + col]
}

/// A geographic position: `(longitude, latitude)` in degrees.
pub type LonLat = (f32, f32);

/// A few well-known cities, `(longitude, latitude)`, handy for connections.
pub mod cities {
    /// San Francisco.
    pub const SAN_FRANCISCO: (f32, f32) = (-122.4, 37.8);
    /// New York.
    pub const NEW_YORK: (f32, f32) = (-74.0, 40.7);
    /// Mexico City.
    pub const MEXICO_CITY: (f32, f32) = (-99.1, 19.4);
    /// São Paulo.
    pub const SAO_PAULO: (f32, f32) = (-46.6, -23.5);
    /// London.
    pub const LONDON: (f32, f32) = (-0.1, 51.5);
    /// Paris.
    pub const PARIS: (f32, f32) = (2.35, 48.86);
    /// Lagos.
    pub const LAGOS: (f32, f32) = (3.4, 6.5);
    /// Cairo.
    pub const CAIRO: (f32, f32) = (31.2, 30.0);
    /// Nairobi.
    pub const NAIROBI: (f32, f32) = (36.8, -1.3);
    /// Dubai.
    pub const DUBAI: (f32, f32) = (55.3, 25.2);
    /// Mumbai.
    pub const MUMBAI: (f32, f32) = (72.9, 19.1);
    /// Singapore.
    pub const SINGAPORE: (f32, f32) = (103.8, 1.35);
    /// Tokyo.
    pub const TOKYO: (f32, f32) = (139.7, 35.7);
    /// Sydney.
    pub const SYDNEY: (f32, f32) = (151.2, -33.9);
    /// Moscow.
    pub const MOSCOW: (f32, f32) = (37.6, 55.8);
}

/// A world map drawn as a grid of dots, with glowing arcs that fly between cities one after
/// another and pulsing markers where they land.
///
/// ```no_run
/// # let ui: &mut egui::Ui = todo!();
/// use glowkit_backgrounds::{DottedMap, cities};
/// ui.add(DottedMap::new().connections(&[(cities::LONDON, cities::NEW_YORK), (cities::LONDON, cities::TOKYO)]));
/// ```
#[must_use = "add it with `ui.add(...)` or call `.paint(ui, rect)`"]
#[derive(Clone, Debug)]
pub struct DottedMap {
    base: Base,
    links: Vec<(LonLat, LonLat)>,
    dot_color: Option<Color32>,
    arc_color: Option<Color32>,
    spacing: f32,
    lat_range: (f32, f32),
    speed: f32,
}

impl Default for DottedMap {
    fn default() -> Self {
        Self::new()
    }
}

impl DottedMap {
    /// A dotted world map with a default set of intercontinental arcs.
    pub fn new() -> Self {
        use cities::*;
        Self {
            base: Base::default(),
            links: vec![
                (SAN_FRANCISCO, NEW_YORK),
                (NEW_YORK, LONDON),
                (LONDON, DUBAI),
                (DUBAI, SINGAPORE),
                (SINGAPORE, SYDNEY),
                (SINGAPORE, TOKYO),
                (MOSCOW, TOKYO),
                (NEW_YORK, SAO_PAULO),
                (SAO_PAULO, LAGOS),
                (LAGOS, NAIROBI),
                (PARIS, MOSCOW),
                (MUMBAI, NAIROBI),
            ],
            dot_color: None,
            arc_color: None,
            spacing: 2.4,
            lat_range: (-56.0, 78.0),
            speed: 1.0,
        }
    }
    /// Arcs to animate, as `((lon, lat), (lon, lat))` pairs (see [`cities`]).
    pub fn connections(mut self, links: &[(LonLat, LonLat)]) -> Self {
        self.links = links.to_vec();
        self
    }
    /// Color of the land dots (default: a faint theme foreground).
    pub fn dot_color(mut self, c: Color32) -> Self {
        self.dot_color = Some(c);
        self
    }
    /// Color of the arcs and markers (default: the theme accent).
    pub fn arc_color(mut self, c: Color32) -> Self {
        self.arc_color = Some(c);
        self
    }
    /// Degrees between dots (default 2.4; smaller is denser).
    pub fn spacing(mut self, degrees: f32) -> Self {
        self.spacing = degrees.clamp(1.0, 10.0);
        self
    }
    /// Latitude range shown, south to north (default -56..78, which skips Antarctica).
    pub fn lat_range(mut self, south: f32, north: f32) -> Self {
        self.lat_range = (south.clamp(-90.0, 89.0), north.clamp(south + 1.0, 90.0));
        self
    }
    /// Animation speed multiplier (default 1).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// Paint the map, fitted and centered in `rect`.
    pub fn paint(&self, ui: &Ui, rect: Rect) {
        let Some(f) = self.base.begin(ui, rect, "glowkit_dotted_map") else { return };
        let t = &f.theme;
        let dark = f.dark();
        let (south, north) = self.lat_range;
        let span = vec2(360.0, north - south);
        let k = (rect.width() / span.x).min(rect.height() / span.y) * 0.96;
        let map = Rect::from_center_size(rect.center(), span * k);
        let proj = |(lon, lat): (f32, f32)| pos2(map.min.x + (lon + 180.0) * k, map.min.y + (north - lat) * k);
        let dot = self.dot_color.unwrap_or_else(|| t.foreground.gamma_multiply(if dark { 0.26 } else { 0.3 }));
        let arc_c = self.arc_color.unwrap_or(t.accent);
        let time = f.time * self.speed;
        // land dots (one mesh; tiny squares with rounded look at this size)
        let step = self.spacing;
        let r = (step * k * 0.3).clamp(0.7, 3.5);
        let mut mesh = Mesh::default();
        let cols = (360.0 / step) as i32;
        let rows = (span.y / step) as i32;
        for j in 0..=rows {
            let lat = north - j as f32 * step;
            for i in 0..cols {
                let lon = -180.0 + (i as f32 + 0.5) * step;
                if !land_at(lon, lat) {
                    continue;
                }
                let c = proj((lon, lat));
                // a slow shimmer across the map
                let tw = 0.75 + 0.25 * (lon * 0.05 + lat * 0.07 + time * 0.8).sin() * h01(i as u32, j as u32);
                let col = dot.gamma_multiply(tw);
                let base = mesh.vertices.len() as u32;
                mesh.colored_vertex(c, col);
                for s in 0..6 {
                    let a = s as f32 / 6.0 * std::f32::consts::TAU;
                    mesh.colored_vertex(c + vec2(a.cos(), a.sin()) * r, col);
                }
                for s in 0..6u32 {
                    mesh.add_triangle(base, base + 1 + s, base + 1 + (s + 1) % 6);
                }
            }
        }
        f.painter.add(Shape::mesh(mesh));
        // arcs: each flies in turn, lingers, then fades; several are in flight at once
        let p = &f.painter;
        let n = self.links.len();
        let slot = 0.9;
        let life = 3.6;
        let cycle = n as f32 * slot;
        for (i, &(a, b)) in self.links.iter().enumerate() {
            let pa = proj(a);
            let pb = proj(b);
            let local = (time - i as f32 * slot).rem_euclid(cycle.max(life + 0.5));
            if local > life {
                // resting: keep faint markers at the endpoints
                for q in [pa, pb] {
                    if map.contains(q) {
                        p.circle_filled(q, 2.2, arc_c.gamma_multiply(0.45));
                    }
                }
                continue;
            }
            let draw = ease(Easing::InOutCubic, (local / 1.4).min(1.0));
            let fade = 1.0 - ease(Easing::InCubic, ((local - 2.6) / 1.0).clamp(0.0, 1.0));
            let mid = pa.lerp(pb, 0.5);
            let lift = (pa.distance(pb) * 0.32).min(map.height() * 0.4);
            let ctrl = mid - vec2(0.0, lift);
            let pts = quad_bezier(pa, ctrl, pb, 48);
            let m = ((pts.len() - 1) as f32 * draw).ceil() as usize;
            if m >= 1 {
                let seg = &pts[..=m.min(pts.len() - 1)];
                gradient_polyline(p, seg, false, 4.0, |u| light(arc_c, 0.12 * fade * u, dark));
                gradient_polyline(p, seg, false, 1.6, |u| {
                    let head = if dark { glowkit_core::mix(arc_c, Color32::WHITE, u * 0.5) } else { arc_c };
                    head.gamma_multiply(fade * (0.15 + 0.85 * u))
                });
                // travelling spark at the head
                if draw < 1.0 {
                    let h = seg[seg.len() - 1];
                    glow(p, h, 12.0, light(arc_c, 0.8 * fade, dark));
                    p.circle_filled(h, 2.0, if dark { Color32::WHITE } else { arc_c });
                }
            }
            // markers: origin always, destination once reached (with a ripple)
            p.circle_filled(pa, 2.6, arc_c.gamma_multiply(0.6 + 0.4 * fade));
            if draw >= 1.0 {
                let since = local - 1.4;
                let ring = (since / 1.2).clamp(0.0, 1.0);
                p.circle_stroke(pb, 3.0 + ring * 14.0, egui::Stroke::new(1.5, arc_c.gamma_multiply((1.0 - ring) * fade)));
                p.circle_filled(pb, 2.8, arc_c.gamma_multiply(0.6 + 0.4 * fade));
            }
        }
    }
}

background!(DottedMap);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continents_are_recognisable() {
        // land
        for (name, lon, lat) in [
            ("Paris", 2.3, 48.8),
            ("Sahara", 10.0, 23.0),
            ("Brazil", -50.0, -10.0),
            ("Kansas", -98.0, 38.0),
            ("Siberia", 100.0, 62.0),
            ("Australia", 134.0, -25.0),
            ("India", 78.0, 20.0),
            ("Greenland", -40.0, 72.0),
        ] {
            assert!(is_land(lon, lat), "{name} should be land");
        }
        // water
        for (name, lon, lat) in [
            ("Atlantic", -35.0, 30.0),
            ("Pacific", -150.0, 0.0),
            ("Indian Ocean", 75.0, -20.0),
            ("Mediterranean", 18.0, 35.0),
            ("Hudson Bay", -86.0, 60.0),
            ("Black Sea", 34.0, 43.5),
            ("Caspian", 51.0, 42.0),
            ("Gulf of Mexico", -90.0, 25.0),
        ] {
            assert!(!is_land(lon, lat), "{name} should be water");
        }
    }
}
