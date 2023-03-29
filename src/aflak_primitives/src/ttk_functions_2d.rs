pub use crate::unit::{
    CriticalPoints, DerivedUnit, Dimensioned, Separatrices1Cell, Separatrices1Point, Topology,
    Unit, WcsArray,
};
pub use crate::{IOErr, IOValue, PersistencePairs};
use ndarray::{Array, Dimension};
use std::slice;
use ttk_sys::Ttk_rs;

pub(crate) fn run_ttk_persistence_pairs(image: &WcsArray) -> Result<IOValue, IOErr> {
    dim_is!(image, 2)?;
    let image_val = image.scalar();
    let dim = image_val.dim();
    let dim = dim.as_array_view();
    let mut data = Vec::new();
    for i in image_val {
        data.push(*i);
    }
    let data_ptr = data.as_mut_ptr();
    let mut birth = Vec::<i32>::with_capacity(image_val.len());
    let mut death = Vec::<i32>::with_capacity(image_val.len());
    let birth_ptr = birth.as_mut_ptr();
    let death_ptr = death.as_mut_ptr();
    let mut act_pair = Vec::new();
    let mut len = 0;

    let mut node = Vec::<i32>::with_capacity(500000);
    let mut arcs = Vec::<i32>::with_capacity(500000 * 3);
    let nodes_ptr = node.as_mut_ptr();
    let arcs_ptr = arcs.as_mut_ptr();
    let mut nodes_len = 0;
    let mut arcs_n = 0;
    let mut arcs_len = 0;
    let mut act_nodes = Vec::new();
    let mut act_arcs = Vec::new();
    unsafe {
        let mut my_ttk = Ttk_rs::new();
        my_ttk.compute_persistence_pairs(
            data_ptr,
            data.len() as u32,
            dim[0] as u32,
            dim[1] as u32,
            birth_ptr,
            death_ptr,
            &mut len,
            nodes_ptr,
            arcs_ptr,
            &mut nodes_len,
            &mut arcs_n,
            &mut arcs_len,
        );
        let birth = slice::from_raw_parts(birth_ptr, len as usize).to_vec();
        let death = slice::from_raw_parts(death_ptr, len as usize).to_vec();
        for i in 0..len {
            act_pair.push((
                *(birth.get(i as usize).unwrap()),
                *(death.get(i as usize).unwrap()),
            ));
        }
        let nodes = slice::from_raw_parts(nodes_ptr, nodes_len as usize).to_vec();
        let arcs = slice::from_raw_parts(arcs_ptr, arcs_len as usize * 3).to_vec();
        for i in 0..nodes_len {
            act_nodes.push(*(nodes.get(i as usize).unwrap()));
            //println!("nodes: {}", *(nodes.get(i as usize).unwrap()));
        }
        let mut counter = 0;
        for _ in 0..arcs_n {
            let from_node = *(arcs.get(counter).unwrap());
            let to_node = *(arcs.get(counter + 1).unwrap());
            let region_size = *(arcs.get(counter + 2).unwrap());
            counter += 3;
            let mut region = Vec::new();
            for _ in 0..region_size {
                region.push(*(arcs.get(counter).unwrap()));
                counter += 1;
            }
            act_arcs.push((from_node, to_node, region_size, region));
            /*println!(
                "arcs: {}->{} #{}",
                *(arcs.get(i as usize * 3).unwrap()),
                *(arcs.get(i as usize * 3 + 1).unwrap()),
                *(arcs.get(i as usize * 3 + 2).unwrap())
            );*/
        }
        for i in 0..len {
            act_pair.push((
                *(birth.get(i as usize).unwrap()),
                *(death.get(i as usize).unwrap()),
            ));
        }
    }
    let mut pp = Vec::<(i32, i32, f32, f32, usize)>::new();
    for (i, j) in act_pair {
        pp.push((
            i,
            j,
            *data.get(i as usize).unwrap(),
            *data.get(j as usize).unwrap(),
            0,
        ));
    }
    Ok(IOValue::PersistencePairs(PersistencePairs::Pairs(pp)))
}

pub(crate) fn run_ttk_persistence_pairs_region(image: &WcsArray) -> Result<IOValue, IOErr> {
    dim_is!(image, 2)?;
    let image_val = image.scalar();
    let dim = image_val.dim();
    let dim = dim.as_array_view();
    let mut data = Vec::new();
    for i in image_val {
        data.push(*i);
    }
    let data_ptr = data.as_mut_ptr();
    let mut birth = Vec::<i32>::with_capacity(image_val.len());
    let mut death = Vec::<i32>::with_capacity(image_val.len());
    let birth_ptr = birth.as_mut_ptr();
    let death_ptr = death.as_mut_ptr();
    let mut act_pair = Vec::new();
    let mut len = 0;

    let mut node = Vec::<i32>::with_capacity(500000);
    let mut arcs = Vec::<i32>::with_capacity(500000 * 3);
    let nodes_ptr = node.as_mut_ptr();
    let arcs_ptr = arcs.as_mut_ptr();
    let mut nodes_len = 0;
    let mut arcs_n = 0;
    let mut arcs_len = 0;
    let mut act_nodes = Vec::new();
    let mut act_arcs = Vec::new();
    unsafe {
        let mut my_ttk = Ttk_rs::new();
        my_ttk.compute_persistence_pairs(
            data_ptr,
            data.len() as u32,
            dim[0] as u32,
            dim[1] as u32,
            birth_ptr,
            death_ptr,
            &mut len,
            nodes_ptr,
            arcs_ptr,
            &mut nodes_len,
            &mut arcs_n,
            &mut arcs_len,
        );
        let birth = slice::from_raw_parts(birth_ptr, len as usize).to_vec();
        let death = slice::from_raw_parts(death_ptr, len as usize).to_vec();
        for i in 0..len {
            act_pair.push((
                *(birth.get(i as usize).unwrap()),
                *(death.get(i as usize).unwrap()),
            ));
        }
        let nodes = slice::from_raw_parts(nodes_ptr, nodes_len as usize).to_vec();
        let arcs = slice::from_raw_parts(arcs_ptr, arcs_len as usize * 3).to_vec();
        for i in 0..nodes_len {
            act_nodes.push(*(nodes.get(i as usize).unwrap()));
            //println!("nodes: {}", *(nodes.get(i as usize).unwrap()));
        }
        let mut counter = 0;
        for _ in 0..arcs_n {
            let from_node = *(arcs.get(counter).unwrap());
            let to_node = *(arcs.get(counter + 1).unwrap());
            let region_size = *(arcs.get(counter + 2).unwrap());
            counter += 3;
            let mut region = Vec::new();
            for _ in 0..region_size {
                region.push(*(arcs.get(counter).unwrap()));
                counter += 1;
            }
            act_arcs.push((from_node, to_node, region_size, region));
            /*println!(
                "arcs: {}->{} #{}",
                *(arcs.get(i as usize * 3).unwrap()),
                *(arcs.get(i as usize * 3 + 1).unwrap()),
                *(arcs.get(i as usize * 3 + 2).unwrap())
            );*/
        }
        //println!("arc_arcs: {:?}", act_arcs);
        for i in 0..len {
            act_pair.push((
                *(birth.get(i as usize).unwrap()),
                *(death.get(i as usize).unwrap()),
            ));
        }
    }
    let mut image = image.clone();
    let mut counter = 0;
    for d in image.scalar_mut().iter_mut() {
        let mut findflag = false;
        for (_, a) in act_arcs.iter().enumerate() {
            for region in &a.3 {
                //println!("region: {}, counter: {}", region, counter);
                if *region == counter {
                    *d = a.2 as f32;
                    findflag = true;
                    //println!("Found");
                    break;
                }
            }
        }
        if !findflag {
            //println!("Not Found");
            *d = std::f32::NAN;
        }
        counter += 1;
    }
    Ok(IOValue::Image(image))
}

pub(crate) fn run_select_the_most_pairs_using_sigma(
    pp: PersistencePairs,
    kappa: f32,
) -> Result<IOValue, IOErr> {
    let PersistencePairs::Pairs(mut data) = pp;
    let mut average = 0.0;
    for d in &data {
        average += d.3 - d.2;
    }
    average /= data.len() as f32;
    let mut sigma = 0.0;
    for d in &data {
        let xi = d.3 - d.2;
        sigma += (xi - average) * (xi - average);
    }
    sigma /= data.len() as f32;
    sigma = sigma.sqrt();
    data.retain(|d| {
        d.3 - d.2 > average
            && ((d.3 - d.2) - average) * ((d.3 - d.2) - average) > kappa * kappa * sigma * sigma
    });
    Ok(IOValue::PersistencePairs(PersistencePairs::Pairs(data)))
}

pub(crate) fn run_ttk_simplification(
    image: &WcsArray,
    pp: PersistencePairs,
) -> Result<IOValue, IOErr> {
    dim_is!(image, 2)?;
    let mut out = image.clone();
    let image_val = image.scalar();
    let dim = image_val.dim();
    let dim = dim.as_array_view();
    let mut data = Vec::new();
    let mut authorized_birth = Vec::new();
    let mut authorized_death = Vec::new();
    let PersistencePairs::Pairs(pairs) = pp;
    for i in image_val {
        data.push(*i);
    }
    for (b, d, _, _, _) in &pairs {
        authorized_birth.push(*b);
        authorized_death.push(*d);
    }
    let data_ptr = data.as_mut_ptr();
    let authorized_birth_ptr = authorized_birth.as_mut_ptr();
    let authorized_death_ptr = authorized_death.as_mut_ptr();

    //critical_points
    let mut cp_len = 0;
    const MAX_CRITICAL_POINTS: usize = 1024;
    let mut cp_point_types = Vec::<u32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_coordx = Vec::<f32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_coordy = Vec::<f32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_value = Vec::<f32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_cellid = Vec::<u32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_pl_vertex_identifier = Vec::<u32>::with_capacity(MAX_CRITICAL_POINTS);
    let mut cp_manifold_size = Vec::<u32>::with_capacity(MAX_CRITICAL_POINTS);

    let cp_point_types_ptr = cp_point_types.as_mut_ptr();
    let cp_coordx_ptr = cp_coordx.as_mut_ptr();
    let cp_coordy_ptr = cp_coordy.as_mut_ptr();
    let cp_value_ptr = cp_value.as_mut_ptr();
    let cp_cellid_ptr = cp_cellid.as_mut_ptr();
    let cp_pl_vertex_identifier_ptr = cp_pl_vertex_identifier.as_mut_ptr();
    let cp_manifold_size_ptr = cp_manifold_size.as_mut_ptr();

    //separatrices1_points
    let mut sp_len = 0;
    const MAX_SEPARATRICES1_POINTS: usize = 32768;
    let mut sp_id = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_POINTS);
    let mut sp_coordx = Vec::<f32>::with_capacity(MAX_SEPARATRICES1_POINTS);
    let mut sp_coordy = Vec::<f32>::with_capacity(MAX_SEPARATRICES1_POINTS);
    let mut sp_point_type = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_POINTS);
    let mut sp_cellid = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_POINTS);

    let sp_id_ptr = sp_id.as_mut_ptr();
    let sp_coordx_ptr = sp_coordx.as_mut_ptr();
    let sp_coordy_ptr = sp_coordy.as_mut_ptr();
    let sp_point_type_ptr = sp_point_type.as_mut_ptr();
    let sp_cellid_ptr = sp_cellid.as_mut_ptr();

    //separatrices1_cells
    let mut sc_len = 0;
    const MAX_SEPARATRICES1_CELLS: usize = 32768;
    let mut sc_id = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_source = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_dest = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_connectivity_s = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_connectivity_d = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_separatrix_id = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_separatrix_type = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_f_maxima = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_f_minima = Vec::<u32>::with_capacity(MAX_SEPARATRICES1_CELLS);
    let mut sc_f_diff = Vec::<f32>::with_capacity(MAX_SEPARATRICES1_CELLS);

    let sc_id_ptr = sc_id.as_mut_ptr();
    let sc_source_ptr = sc_source.as_mut_ptr();
    let sc_dest_ptr = sc_dest.as_mut_ptr();
    let sc_connectivity_s_ptr = sc_connectivity_s.as_mut_ptr();
    let sc_connectivity_d_ptr = sc_connectivity_d.as_mut_ptr();
    let sc_separatrix_id_ptr = sc_separatrix_id.as_mut_ptr();
    let sc_separatrix_type_ptr = sc_separatrix_type.as_mut_ptr();
    let sc_f_maxima_ptr = sc_f_maxima.as_mut_ptr();
    let sc_f_minima_ptr = sc_f_minima.as_mut_ptr();
    let sc_f_diff_ptr = sc_f_diff.as_mut_ptr();

    //morsesmale
    let mut morsesmale = Vec::<f32>::with_capacity(data.len());
    let morsesmale_ptr = morsesmale.as_mut_ptr();

    unsafe {
        let mut my_ttk = Ttk_rs::new();
        my_ttk.simplification(
            data_ptr,
            data.len() as u32,
            dim[1] as u32,
            dim[0] as u32,
            authorized_birth_ptr,
            authorized_death_ptr,
            pairs.len() as u32,
            cp_point_types_ptr,
            cp_coordx_ptr,
            cp_coordy_ptr,
            cp_value_ptr,
            cp_cellid_ptr,
            cp_pl_vertex_identifier_ptr,
            cp_manifold_size_ptr,
            &mut cp_len,
            sp_id_ptr,
            sp_coordx_ptr,
            sp_coordy_ptr,
            sp_point_type_ptr,
            sp_cellid_ptr,
            &mut sp_len,
            sc_id_ptr,
            sc_source_ptr,
            sc_dest_ptr,
            sc_connectivity_s_ptr,
            sc_connectivity_d_ptr,
            sc_separatrix_id_ptr,
            sc_separatrix_type_ptr,
            sc_f_maxima_ptr,
            sc_f_minima_ptr,
            sc_f_diff_ptr,
            &mut sc_len,
            morsesmale_ptr,
        );
        println!("After Simplification Rust part");
        println!(
            "cp_len: {}, MAX_CRITICAL_POINTS: {}",
            cp_len, MAX_CRITICAL_POINTS
        );
        if cp_len >= MAX_CRITICAL_POINTS as u32 {
            println!("maybe segfault!");
        }
        let cp_point_types = slice::from_raw_parts(cp_point_types_ptr, cp_len as usize).to_vec();
        let cp_coordx = slice::from_raw_parts(cp_coordx_ptr, cp_len as usize).to_vec();
        let cp_coordy = slice::from_raw_parts(cp_coordy_ptr, cp_len as usize).to_vec();
        let cp_value = slice::from_raw_parts(cp_value_ptr, cp_len as usize).to_vec();
        let cp_cellid = slice::from_raw_parts(cp_cellid_ptr, cp_len as usize).to_vec();
        let cp_pl_vertex_identifier =
            slice::from_raw_parts(cp_pl_vertex_identifier_ptr, cp_len as usize).to_vec();
        let cp_manifold_size =
            slice::from_raw_parts(cp_manifold_size_ptr, cp_len as usize).to_vec();

        println!(
            "sp_len: {}, MAX_SEPARATRICES1_POINTS: {}",
            sp_len, MAX_SEPARATRICES1_POINTS
        );
        if sp_len >= MAX_SEPARATRICES1_POINTS as u32 {
            println!("maybe segfault!");
        }

        let sp_id = slice::from_raw_parts(sp_id_ptr, sp_len as usize).to_vec();
        let sp_coordx = slice::from_raw_parts(sp_coordx_ptr, sp_len as usize).to_vec();
        let sp_coordy = slice::from_raw_parts(sp_coordy_ptr, sp_len as usize).to_vec();
        let sp_point_type = slice::from_raw_parts(sp_point_type_ptr, sp_len as usize).to_vec();
        let sp_cellid = slice::from_raw_parts(sp_cellid_ptr, sp_len as usize).to_vec();

        println!(
            "sc_len: {}, MAX_SEPARATRICES1_CELLS: {}",
            sc_len, MAX_SEPARATRICES1_CELLS
        );
        if sc_len >= MAX_SEPARATRICES1_CELLS as u32 {
            println!("maybe segfault!");
        }

        let sc_id = slice::from_raw_parts(sc_id_ptr, sc_len as usize).to_vec();
        let sc_source = slice::from_raw_parts(sc_source_ptr, sc_len as usize).to_vec();
        let sc_dest = slice::from_raw_parts(sc_dest_ptr, sc_len as usize).to_vec();
        let sc_connectivity_s =
            slice::from_raw_parts(sc_connectivity_s_ptr, sc_len as usize).to_vec();
        let sc_connectivity_d =
            slice::from_raw_parts(sc_connectivity_d_ptr, sc_len as usize).to_vec();
        let sc_separatrix_id =
            slice::from_raw_parts(sc_separatrix_id_ptr, sc_len as usize).to_vec();
        let sc_separatrix_type =
            slice::from_raw_parts(sc_separatrix_type_ptr, sc_len as usize).to_vec();
        let sc_f_maxima = slice::from_raw_parts(sc_f_maxima_ptr, sc_len as usize).to_vec();
        let sc_f_minima = slice::from_raw_parts(sc_f_minima_ptr, sc_len as usize).to_vec();
        let sc_f_diff = slice::from_raw_parts(sc_f_diff_ptr, sc_len as usize).to_vec();

        let morsesmale = slice::from_raw_parts(morsesmale_ptr, data.len()).to_vec();
        println!("After Read from_raw_parts");
        let mut critical_points = Vec::new();
        let mut separatrices1_points = Vec::new();
        let mut separatrices1_cells = Vec::new();
        println!("Compute CP");
        for i in 0..cp_len as usize {
            let cp = CriticalPoints::new(
                cp_point_types[i] as usize,
                (cp_coordx[i], cp_coordy[i], 0.0),
                cp_value[i],
                cp_cellid[i] as usize,
                cp_pl_vertex_identifier[i] as usize,
                cp_manifold_size[i] as usize,
            );
            critical_points.push(cp);
        }
        println!("Compute SP");
        for i in 0..sp_len as usize {
            let sp = Separatrices1Point::new(
                sp_id[i] as usize,
                (sp_coordx[i], sp_coordy[i], 0.0),
                sp_point_type[i] as usize,
                sp_cellid[i] as usize,
            );
            separatrices1_points.push(sp);
        }
        println!("Compute SC");
        for i in 0..sc_len as usize {
            let sc = Separatrices1Cell::new(
                sc_id[i] as usize,
                sc_source[i] as usize,
                sc_dest[i] as usize,
                (sc_connectivity_s[i] as usize, sc_connectivity_d[i] as usize),
                sc_separatrix_id[i] as usize,
                sc_separatrix_type[i] as usize,
                sc_f_maxima[i] as usize,
                sc_f_minima[i] as usize,
                sc_f_diff[i],
            );
            separatrices1_cells.push(sc);
        }
        let topology = Topology::new(
            critical_points,
            separatrices1_points,
            separatrices1_cells,
            morsesmale,
        );
        out.set_topology(Some(topology));
        println!("All done!");
    }
    Ok(IOValue::Image(out))
}

pub(crate) fn run_ttk_get_simplified(
    image: &WcsArray,
    pp: PersistencePairs,
) -> Result<IOValue, IOErr> {
    dim_is!(image, 2)?;
    let image_val = image.scalar();
    let dim = image_val.dim();
    let dim = dim.as_array_view();
    let mut data = Vec::new();
    let mut authorized_birth = Vec::new();
    let mut authorized_death = Vec::new();
    let PersistencePairs::Pairs(pairs) = pp;
    for i in image_val {
        data.push(*i);
    }
    for (b, d, _, _, _) in &pairs {
        authorized_birth.push(*b);
        authorized_death.push(*d);
    }
    let data_ptr = data.as_mut_ptr();
    let authorized_birth_ptr = authorized_birth.as_mut_ptr();
    let authorized_death_ptr = authorized_death.as_mut_ptr();

    let mut simplified = Vec::with_capacity(data.len());
    let simplified_ptr = simplified.as_mut_ptr();
    let mut simplified_len = 0;
    let mut act_simplified = Vec::new();
    unsafe {
        let mut my_ttk = Ttk_rs::new();
        my_ttk.get_simplified(
            data_ptr,
            data.len() as u32,
            dim[1] as u32,
            dim[0] as u32,
            authorized_birth_ptr,
            authorized_death_ptr,
            pairs.len() as u32,
            simplified_ptr,
            &mut simplified_len,
        );
        println!("After Simplification Rust part");
        let simplified = slice::from_raw_parts(simplified_ptr, simplified_len as usize).to_vec();
        for d in simplified {
            act_simplified.push(d);
        }
    }
    let mut image = image.clone();
    let mut counter = 0;
    for d in image.scalar_mut().iter_mut() {
        *d = *act_simplified.get(counter).unwrap();
        counter += 1;
    }
    Ok(IOValue::Image(image))
}

pub(crate) fn run_visualize_morsesmale(image: &WcsArray) -> Result<IOValue, IOErr> {
    if let Some(topology) = image.topology() {
        let dim = image.scalar().dim();
        let dim = dim.as_array_view();
        let img = Array::from_shape_vec((dim[0], dim[1]), topology.morsesmale.clone()).unwrap();
        Ok(IOValue::Image(WcsArray::from_array(Dimensioned::new(
            img.into_dyn(),
            Unit::None,
        ))))
    } else {
        Err(IOErr::UnexpectedInput(format!("No topology")))
    }
}
