use std::collections::{BinaryHeap, HashMap, HashSet};
use std::time::Duration;

use geo::{Coord, EuclideanLength, Point};
use serde::Serialize;
use utils::PriorityQueueItem;

use crate::graph::{AmenityID, Graph, IntersectionID};
use crate::{Person, Request};

#[derive(Serialize)]
pub struct POI {
    pub osm_url: String,
    pub point: Point,
    pub kind: String,
    pub name: Option<String>,

    /// (Name, cost in seconds)
    pub times_per_person: Vec<(String, u64)>,
}

pub fn find_pois(graph: &Graph, req: Request) -> Vec<POI> {
    let mut pois: HashMap<AmenityID, POI> = HashMap::new();

    for person in req.people {
        for (a, cost) in get_costs(graph, &person) {
            let amenity = &graph.amenities[a.0];
            pois.entry(a)
                .or_insert_with(|| POI {
                    osm_url: amenity.osm_id.to_string(),
                    point: graph.mercator.to_wgs84(&amenity.point),
                    kind: amenity.kind.clone(),
                    name: amenity.name.clone(),
                    times_per_person: Vec::new(),
                })
                .times_per_person
                .push((person.name.clone(), cost.as_secs()));
        }
    }

    pois.into_values().collect()
}

fn get_costs(graph: &Graph, person: &Person) -> HashMap<AmenityID, Duration> {
    // 3 mph in meters/second
    let walking_speed = 1.34112;

    let start = graph
        .closest_intersection
        .nearest_neighbor(&x_y(graph.mercator.pt_to_mercator(Coord {
            x: person.home[0],
            y: person.home[1],
        })))
        .unwrap()
        .data;
    let limit = Duration::from_secs(60 * person.max_time_minutes);

    let mut visited: HashSet<IntersectionID> = HashSet::new();
    let mut cost_per_poi: HashMap<AmenityID, Duration> = HashMap::new();
    let mut queue: BinaryHeap<PriorityQueueItem<Duration, IntersectionID>> = BinaryHeap::new();

    queue.push(PriorityQueueItem::new(Duration::ZERO, start));

    while let Some(current) = queue.pop() {
        if visited.contains(&current.value) {
            continue;
        }
        visited.insert(current.value);
        if current.cost > limit {
            continue;
        }

        for road in graph.roads_per_intersection(current.value) {
            let this_cost =
                Duration::from_secs_f64(road.linestring.euclidean_length() / walking_speed);

            for a in &road.amenities {
                cost_per_poi.insert(*a, current.cost + this_cost);
            }

            queue.push(PriorityQueueItem::new(
                current.cost + this_cost,
                road.other_side(current.value),
            ));
        }
    }

    cost_per_poi
}

fn x_y(c: Coord) -> [f64; 2] {
    [c.x, c.y]
}