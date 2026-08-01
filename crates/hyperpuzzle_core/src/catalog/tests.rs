use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;

use super::*;

fn sleep_ms(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}

#[derive(Debug, PartialEq, Eq)]
struct TestObj {
    id: CatalogId,
    data: usize,
}

impl TestObj {
    fn from_ctx(ctx: BuildCtx, data: usize) -> Self {
        Self {
            id: ctx.id().clone(),
            data,
        }
    }
    fn new(id: &str, data: usize) -> Result<Self> {
        Ok(Self {
            id: id.parse()?,
            data,
        })
    }
}

impl CatalogObject for TestObj {
    fn catalog_type_name() -> &'static str {
        "testobj"
    }

    fn id(&self) -> &CatalogId {
        &self.id
    }
}

#[test]
fn test_catalog_request() -> Result<()> {
    let catalog_builder = CatalogBuilder::new();

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_ref = Arc::clone(&counter);

    catalog_builder.add(Arc::new(Generator::new_constant(
        "a".parse()?,
        move |ctx| {
            counter_ref.fetch_add(1, Relaxed);
            Ok(Arc::new(TestObj::from_ctx(ctx, 42)))
        },
    )))?;

    let catalog = catalog_builder.build()?;

    // Test arg parsing
    assert_eq!(
        "error building testobj `a(15)`: testobj generator `a` requires 0 params; got 1",
        catalog
            .build_blocking::<TestObj>(&"a(15)".parse()?)
            .unwrap_err()
            .to_string()
    );
    assert_eq!(
        "error building testobj `a.bad_subset`: testobj generator `a` has no subset parameter; got `bad_subset`",
        catalog
            .build_blocking::<TestObj>(&"a.bad_subset".parse()?)
            .unwrap_err()
            .to_string()
    );

    // Test caching
    let expected = Arc::new(TestObj::new("a", 42)?);
    assert_eq!(0, counter.load(Relaxed));
    assert_eq!(expected, catalog.build_blocking(&"a".parse()?).unwrap());
    assert_eq!(1, counter.load(Relaxed));
    assert_eq!(expected, catalog.build_blocking(&"a".parse()?).unwrap());
    assert_eq!(1, counter.load(Relaxed));

    Ok(())
}

#[test]
fn test_catalog_cancel_request() -> Result<()> {
    let allow_completion = Arc::new(AtomicBool::new(false));
    let allow_cancel = Arc::new(AtomicBool::new(true));
    let cancels = Arc::new(AtomicUsize::new(0));
    let completions = Arc::new(AtomicUsize::new(0));
    let allow_completion_ref = Arc::clone(&allow_completion);
    let allow_cancel_ref = Arc::clone(&allow_cancel);
    let cancels_ref = Arc::clone(&cancels);
    let completions_ref = Arc::clone(&completions);

    let catalog_builder = CatalogBuilder::new();
    catalog_builder.add(Arc::new(Generator::new_constant(
        "a".parse()?,
        move |ctx| {
            while !allow_completion_ref.load(Relaxed) {
                if allow_cancel_ref.load(Relaxed) && ctx.is_canceled() {
                    cancels_ref.fetch_add(1, Relaxed);
                    ctx.cancel_if_unrequested()?;
                }
                sleep_ms(10);
            }
            completions_ref.fetch_add(1, Relaxed);
            Ok(Arc::new(TestObj::from_ctx(ctx, 10)))
        },
    )))?;
    let catalog = catalog_builder.build()?;

    let expected = Arc::new(TestObj::new("a", 10)?);
    assert_eq!(0, completions.load(Relaxed));

    let request1 = catalog.build::<TestObj>(&"a".parse()?);
    assert_eq!(0, cancels.load(Relaxed));
    drop(request1);
    sleep_ms(50);
    assert_eq!(1, cancels.load(Relaxed));

    let request2 = catalog.build::<TestObj>(&"a".parse()?);
    let request3 = catalog.build::<TestObj>(&"a".parse()?);
    drop(request2);
    sleep_ms(50);
    assert_eq!(1, cancels.load(Relaxed));

    allow_cancel.store(false, Relaxed);
    drop(request3);
    // request is canceled, even though the thread is still running
    sleep_ms(50);
    let request4 = catalog.build::<TestObj>(&"a".parse()?); // start a new request
    sleep_ms(50);
    allow_cancel.store(true, Relaxed);
    sleep_ms(50); // now the thread should have stopped
    assert_eq!(2, cancels.load(Relaxed));

    sleep_ms(50);
    allow_completion.store(true, Relaxed);
    sleep_ms(50);

    assert_eq!(expected, request4.get_blocking().unwrap());
    assert_eq!(2, cancels.load(Relaxed));
    assert_eq!(1, completions.load(Relaxed));

    Ok(())
}

#[test]
fn test_catalog_cancel_subrequest() -> Result<()> {
    let allow_completion = Arc::new(AtomicBool::new(false));
    let allow_cancel = Arc::new(AtomicBool::new(true));
    let cancels = Arc::new(AtomicUsize::new(0));
    let completions = Arc::new(AtomicUsize::new(0));
    let allow_completion_ref = Arc::clone(&allow_completion);
    let allow_cancel_ref = Arc::clone(&allow_cancel);
    let cancels_ref = Arc::clone(&cancels);
    let completions_ref = Arc::clone(&completions);

    let catalog_builder = CatalogBuilder::new();
    catalog_builder.add(Arc::new(Generator::new_constant(
        "a".parse()?,
        move |ctx| {
            ctx.build_str_blocking::<TestObj>("b")
                .map_err(|e| eyre!("{e:#}"))
        },
    )))?;
    catalog_builder.add(Arc::new(Generator::new_constant(
        "b".parse()?,
        move |ctx| {
            while !allow_completion_ref.load(Relaxed) {
                if allow_cancel_ref.load(Relaxed) && ctx.is_canceled() {
                    cancels_ref.fetch_add(1, Relaxed);
                    ctx.cancel_if_unrequested()?;
                }
                sleep_ms(10);
            }
            completions_ref.fetch_add(1, Relaxed);
            Ok(Arc::new(TestObj::from_ctx(ctx, 1234)))
        },
    )))?;
    let catalog = catalog_builder.build()?;

    let expected = Arc::new(TestObj::new("b", 1234)?);
    assert_eq!(0, completions.load(Relaxed));

    let request_a1 = catalog.build::<TestObj>(&"a".parse()?);
    sleep_ms(50);
    assert_eq!(0, cancels.load(Relaxed));
    drop(request_a1); // cancel `a` and `b`
    sleep_ms(50);
    assert_eq!(1, cancels.load(Relaxed));

    let request_a2 = catalog.build::<TestObj>(&"a".parse()?);
    let request_b1 = catalog.build::<TestObj>(&"b".parse()?);
    drop(request_a2); // cancels `a` but *not* `b`
    sleep_ms(50);
    allow_completion.store(true, Relaxed);
    assert_eq!(expected, request_b1.get_blocking().unwrap());

    assert_eq!(expected, catalog.build_blocking(&"a".parse()?).unwrap());

    assert_eq!(1, cancels.load(Relaxed));
    assert_eq!(1, completions.load(Relaxed));

    Ok(())
}
