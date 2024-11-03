/// Something like [`Iterator::for_each`] for slices, but specifies interleaving
/// function, which gets executed after each element except the last one.
///
/// Also allows providing context as the same capture may be shared between both
/// closures which would make borrowchk sad as we borrow mutably here.
pub fn slice_try_for_each_interleaved_with_context<Item, Error, Ctx>(
	slice: &[Item],
	context: &mut Ctx,
	mut item_f: impl FnMut(&mut Ctx, &Item) -> Result<(), Error>,
	mut interleave_f: impl FnMut(&mut Ctx) -> Result<(), Error>,
) -> Result<(), Error> {
	let Some((last, rest)) = slice.split_last() else {
		return Ok(());
	};

	for item in rest {
		item_f(context, item)?;
		interleave_f(context)?;
	}

	item_f(context, last)
}
