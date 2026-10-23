//! Synthetic source-bound simulator execution and recovery controls.
use super::admission::source_bundle;
use super::*;
#[tokio::test]
async fn source_candidate_executes_and_disk_reopened_restore_returns_full_independent_old_state() {
    for namespace in [20, 23] {
        let bundle = source_bundle(namespace);
        let device = synthetic_old_device(&bundle, SimulatorConfig::default());
        let old = device.system2_snapshot();
        let dir = tempfile::tempdir().unwrap();
        let root = std::env::var_os("KNXBENCH_SYNTHETIC_ARTIFACT_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| dir.path().to_path_buf());
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join(format!("source-execution-bundle-namespace{namespace}.json"));
        std::fs::write(&file, serde_json::to_vec(&bundle).unwrap()).unwrap();
        let path = root.join(format!("source-operation-namespace{namespace}"));
        let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
        app::execute_simulated(
            &device,
            &bundle,
            &mut store,
            integrated_timing(),
            &AtomicBool::new(false),
        )
        .await
        .unwrap();
        assert_eq!(store.outcome(), LocalOutcome::Finished);
        assert_ne!(device.system2_snapshot().memory, old.memory);
        assert_eq!(device.system2_snapshot().objects, old.objects);
        assert_eq!(device.memory(15360, 3), [Some(1), Some(2), Some(0xa5)]);
        assert_eq!(device.memory(285, 8), vec![Some(0xa5); 8]);
        assert_eq!(device.memory(293, 1), [Some(0)]);
        assert_eq!(device.memory(615, 8), vec![Some(0xa5); 8]);
        assert_eq!(device.memory(623, 1), [Some(0)]);
        let backup = store.recovery().unwrap().clone();
        assert_eq!(
            backup
                .allocations
                .iter()
                .map(|a| a.data.len())
                .sum::<usize>(),
            48
        );
        assert!(backup
            .allocations
            .iter()
            .all(|a| a.data.iter().all(|b| *b == 0xa5)));
        drop(store);
        drop(bundle);
        let bundle = app::read_bundle(&file).unwrap();
        let mut fresh = OperationStore::open(&path, &bundle.binding).unwrap();
        let frames = device.seen().len();
        assert!(app::restore_simulated(
            &device,
            &bundle,
            &mut fresh,
            integrated_timing(),
            &AtomicBool::new(false),
            "wrong"
        )
        .await
        .is_err());
        assert_eq!(device.seen().len(), frames);
        app::restore_simulated(
            &device,
            &bundle,
            &mut fresh,
            integrated_timing(),
            &AtomicBool::new(false),
            &bundle.restore_phrase(),
        )
        .await
        .unwrap();
        assert_eq!(fresh.outcome(), LocalOutcome::Restored);
        assert_eq!(device.system2_snapshot(), old);
        assert_eq!(fresh.recovery().unwrap(), &backup);
        assert!(app::restore_simulated(
            &device,
            &bundle,
            &mut fresh,
            integrated_timing(),
            &AtomicBool::new(false),
            &bundle.restore_phrase()
        )
        .await
        .is_err());
        drop(fresh);
        assert_eq!(
            OperationStore::open(&path, &bundle.binding)
                .unwrap()
                .outcome(),
            LocalOutcome::Restored
        );
    }
}
