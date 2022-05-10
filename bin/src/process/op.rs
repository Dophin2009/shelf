use std::path::Path;

use shelflib::{
    action::Action,
    op::{
        copy::{CopyOpError, CopyUndoOpError},
        create::{CreateOpError, CreateUndoOpError},
        error::{
            CopyError, CreateError, MkdirError, OpenError, ReadError, RemoveError, RenameError,
            SymlinkError, WriteError,
        },
        journal::JournalOpFinish,
        link::{LinkOpError, LinkUndoOpError},
        mkdir::{MkdirOpError, MkdirUndoOpError},
        rm::{RmOpError, RmUndoOpError},
        write::{WriteOpError, WriteUndoOpError},
        CommandOp, CopyOp, CopyUndoOp, CreateOp, CreateUndoOp, Finish, FunctionOp, LinkOp,
        LinkUndoOp, MkdirOp, MkdirUndoOp, Op, RmOp, RmUndoOp, WriteOp, WriteUndoOp,
    },
};

use super::{describe, Describe, DescribeMode, GraphProcessor};
use crate::output::{
    comb::{sjoin2, sjoin3, sjoin4},
    spath, Pretty, Step,
};
use crate::{ctxpath::CtxPath, output::comb::pretty};

impl<'p, 'g> GraphProcessor<'p, 'g> {
    #[inline]
    pub fn process_op<'lua>(
        &mut self,
        action: &Action<'lua>,
        op: Op,
        path: &CtxPath,
        dest: &Path,
    ) -> Result<(), ()> {
        // TODO: Lots of cloning :(
        match op.clone() {
            Op::Link(iop) => self.process_link_op(action, op, iop, path, dest),
            Op::LinkUndo(iop) => self.process_link_undo_op(action, op, iop, path, dest),
            Op::Copy(iop) => self.process_copy_op(action, op, iop, path, dest),
            Op::CopyUndo(iop) => self.process_copy_undo_op(action, op, iop, path, dest),
            Op::Create(iop) => self.process_create_op(action, op, iop, path, dest),
            Op::CreateUndo(iop) => self.process_create_undo_op(action, op, iop, path, dest),
            Op::Write(iop) => self.process_write_op(action, op, iop, path, dest),
            Op::WriteUndo(iop) => self.process_write_undo_op(action, op, iop, path, dest),
            Op::Mkdir(iop) => self.process_mkdir_op(action, op, iop, path, dest),
            Op::MkdirUndo(iop) => self.process_mkdir_undo_op(action, op, iop, path, dest),
            Op::Rm(iop) => self.process_rm_op(action, op, iop, path, dest),
            Op::RmUndo(iop) => self.process_rm_undo_op(action, op, iop, path, dest),
            Op::Command(iop) => {
                // TODO: Output
                match iop.finish(&self.opts.ctx) {
                    Ok(_fin) => {
                        // TODO: Output
                        Ok(())
                    }
                    Err(_err) => {
                        // TODO: Output
                        Err(())
                    }
                }
            }
            Op::Function(iop) => {
                // TODO: Output
                match iop.finish(&self.opts.ctx) {
                    Ok(_fin) => {
                        // TODO: Output
                        Ok(())
                    }
                    Err(_err) => {
                        // TODO: Output
                        Err(())
                    }
                }
            }
        }
    }

    #[inline]
    pub fn op_append_finish<O>(&mut self, op: O) -> Result<(), O::Error>
    where
        O: Finish,
        O::Output: Into<JournalOpFinish>,
    {
        let mut t = self.journal.lock();
        t.append_finish(op, &self.opts.ctx).map(|_| ())
    }
}

macro_rules! process_op_impl {
    (
        $name:ident, $op_ty:ty, $action:ident, $op:ident, $iop:ident,
        $path:ident, $dest:ident, $err:ident => $out:expr
    ) => {
        #[inline]
        pub fn $name<'lua>(
            &mut self,
            $action: &Action<'lua>,
            $op: Op<'lua>,
            $iop: $op_ty,
            $path: &CtxPath,
            $dest: &Path,
        ) -> Result<(), ()> {
            match self.op_append_finish($iop) {
                Ok(_) => Ok(()),
                Err($err) => {
                    $out;
                    Err(())
                }
            }
        }
    };
}

#[allow(unreachable_code)]
impl<'p, 'g> GraphProcessor<'p, 'g> {
    process_op_impl!(process_link_op, LinkOp,
        action, op, iop, path, dest, err => match err {
            LinkOpError::Symlink(err) => emit_symlink_error(err, action, op, path, dest)
        }
    );

    process_op_impl!(process_link_undo_op, LinkUndoOp,
        action, op, iop, path, dest, err => match err {
            LinkUndoOpError::Remove(err) => emit_remove_error(err, action, op, path, dest)
        }
    );

    process_op_impl!(process_copy_op, CopyOp,
        action, op, iop, path, dest, err => match err {
            CopyOpError::Copy(err) => emit_copy_error(err, action, op, path, dest)
        }
    );

    process_op_impl!(process_copy_undo_op, CopyUndoOp,
        action, op, iop, path, dest, err => match err {
            CopyUndoOpError::Remove(err) => emit_remove_error(err, action, op, path, dest)
        }
    );

    process_op_impl!(process_create_op, CreateOp,
        action, op, iop, path, dest, err => match err {
            CreateOpError::Create(err) => emit_create_error(err, action, op, path, dest)
        }
    );

    process_op_impl!(process_create_undo_op, CreateUndoOp,
        action, op, iop, path, dest, err => match err {
            CreateUndoOpError::Remove(err) => emit_remove_error(err, action, op, path, dest)
        }
    );

    process_op_impl!(process_write_op, WriteOp,
        action, op, iop, path, dest, err => match err {
            WriteOpError::Open(err) => emit_open_error(err, action, op, path, dest),
            WriteOpError::Read(err) => emit_read_error(err, action, op, path, dest),
            WriteOpError::Write(err) => emit_write_error(err, action, op, path, dest),
        }
    );

    process_op_impl!(process_write_undo_op, WriteUndoOp,
        action, op, iop, path, dest, err => match err {
            WriteUndoOpError::Open(err) => emit_open_error(err, action, op, path, dest),
            WriteUndoOpError::Read(err) => emit_read_error(err, action, op, path, dest),
            WriteUndoOpError::Write(err) => emit_write_error(err, action, op, path, dest),
        }
    );

    process_op_impl!(process_mkdir_op, MkdirOp,
        action, op, iop, path, dest, err => match err {
            MkdirOpError::Mkdir(err) => emit_mkdir_error(err, action, op, path, dest)
        }
    );

    process_op_impl!(process_mkdir_undo_op, MkdirUndoOp,
        action, op, iop, path, dest, err => match err {
            MkdirUndoOpError::Remove(err) => emit_remove_error(err, action, op, path, dest)
        }
    );

    process_op_impl!(process_rm_op, RmOp,
        action, op, iop, path, dest, err => match err {
            RmOpError::Rename(err) => emit_rename_error(err, action, op, path, dest)
        }
    );

    process_op_impl!(process_rm_undo_op, RmUndoOp,
        action, op, iop, path, dest, err => match err {
            RmUndoOpError::Rename(err) => emit_rename_error(err, action, op, path, dest)
        }
    );
}

macro_rules! emit_error_impl {
    ($name:ident, $ty:ty: $err:ident => $message:expr) => {
        #[inline]
        fn $name<'lua>(
            $err: $ty,
            action: &Action<'lua>,
            op: Op<'lua>,
            path: &CtxPath,
            dest: &Path,
        ) {
            Step::error()
                .message($message)
                .reason($err.inner)
                .context(op.describe(path, dest, DescribeMode::Error))
                .context(action.describe(path, dest, DescribeMode::Error));
        }
    };
}

emit_error_impl!(emit_symlink_error, SymlinkError:
    err => sjoin2("couldn't symlink to", spath(err.dest))
);

emit_error_impl!(emit_copy_error, CopyError:
    err => sjoin2("couldn't copy to", spath(err.dest))
);

emit_error_impl!(emit_create_error, CreateError:
    err => sjoin2("couldn't create", spath(err.path))
);

emit_error_impl!(emit_mkdir_error, MkdirError:
    err => sjoin2("couldn't create", spath(err.path))
);

emit_error_impl!(emit_open_error, OpenError:
    err => sjoin2("couldn't open", spath(err.path))
);

emit_error_impl!(emit_read_error, ReadError:
    err => sjoin2("couldn't open", spath(err.path))
);

emit_error_impl!(emit_remove_error, RemoveError:
    err => sjoin2("couldn't remove", spath(err.path))
);

emit_error_impl!(emit_rename_error, RenameError:
    err => sjoin4("couldn't rename", spath(err.src), "to", spath(err.dest))
);

emit_error_impl!(emit_write_error, WriteError:
    err => sjoin2("couldn't write to", spath(err.path))
);

impl<'lua> Describe for Op<'lua> {
    #[inline]
    fn describe(&self, path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty {
        match self {
            Op::Link(op) => op.describe(path, dest, mode),
            Op::LinkUndo(op) => op.describe(path, dest, mode),
            Op::Copy(op) => op.describe(path, dest, mode),
            Op::CopyUndo(op) => op.describe(path, dest, mode),
            Op::Create(op) => op.describe(path, dest, mode),
            Op::CreateUndo(op) => op.describe(path, dest, mode),
            Op::Write(op) => op.describe(path, dest, mode),
            Op::WriteUndo(op) => op.describe(path, dest, mode),
            Op::Mkdir(op) => op.describe(path, dest, mode),
            Op::MkdirUndo(op) => op.describe(path, dest, mode),
            Op::Rm(op) => op.describe(path, dest, mode),
            Op::RmUndo(op) => op.describe(path, dest, mode),
            Op::Command(op) => op.describe(path, dest, mode),
            Op::Function(op) => op.describe(path, dest, mode),
        }
    }
}

impl Describe for LinkOp {
    #[inline]
    fn describe(&self, path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty {
        let src = describe::path_relative(&self.src, path);
        let dest = describe::dest_relative(&self.dest, dest);
        sjoin4(
            "creating symlink from",
            describe::mode_spath(src, mode),
            "to",
            describe::mode_spath(dest, mode),
        )
    }
}

impl Describe for LinkUndoOp {
    #[inline]
    fn describe(&self, path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty {
        let src = describe::path_relative(&self.src, path);
        let dest = describe::dest_relative(&self.dest, dest);
        sjoin4(
            "undoing symlink from",
            describe::mode_spath(src, mode),
            "to",
            describe::mode_spath(dest, mode),
        )
    }
}

impl Describe for CopyOp {
    #[inline]
    fn describe(&self, path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty {
        let src = describe::path_relative(&self.src, path);
        let dest = describe::dest_relative(&self.dest, dest);
        sjoin4(
            "copying file from",
            describe::mode_spath(src, mode),
            "to",
            describe::mode_spath(dest, mode),
        )
    }
}

impl Describe for CopyUndoOp {
    #[inline]
    fn describe(&self, path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty {
        let src = describe::path_relative(&self.src, path);
        let dest = describe::dest_relative(&self.dest, dest);
        sjoin4(
            "undoing copy file from",
            describe::mode_spath(src, mode),
            "to",
            describe::mode_spath(dest, mode),
        )
    }
}

impl Describe for CreateOp {
    #[inline]
    fn describe(&self, _path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty {
        let path = describe::dest_relative(&self.path, dest);
        sjoin2("creating file", describe::mode_spath(path, mode))
    }
}

impl Describe for CreateUndoOp {
    #[inline]
    fn describe(&self, _path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty {
        let path = describe::dest_relative(&self.path, dest);
        sjoin2("removing created file", describe::mode_spath(path, mode))
    }
}

impl Describe for WriteOp {
    #[inline]
    fn describe(&self, _path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty {
        let path = describe::dest_relative(&self.path, dest);
        sjoin2("writing to", describe::mode_spath(path, mode))
    }
}

impl Describe for WriteUndoOp {
    #[inline]
    fn describe(&self, _path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty {
        let path = describe::dest_relative(&self.path, dest);
        sjoin2("undoing write to", describe::mode_spath(path, mode))
    }
}

impl Describe for MkdirOp {
    #[inline]
    fn describe(&self, _path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty {
        let path = describe::dest_relative(&self.path, dest);
        sjoin2("creating directory", describe::mode_spath(path, mode))
    }
}

impl Describe for MkdirUndoOp {
    #[inline]
    fn describe(&self, _path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty {
        let path = describe::dest_relative(&self.path, dest);
        sjoin2(
            "removing created directory",
            describe::mode_spath(path, mode),
        )
    }
}

impl Describe for RmOp {
    #[inline]
    fn describe(&self, _path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty {
        let path = describe::dest_relative(&self.path, dest);
        sjoin2("removing file", describe::mode_spath(path, mode))
    }
}

impl Describe for RmUndoOp {
    #[inline]
    fn describe(&self, _path: &CtxPath, dest: &Path, mode: DescribeMode) -> Pretty {
        let path = describe::dest_relative(&self.path, dest);
        sjoin2("restoring removed file", describe::mode_spath(path, mode))
    }
}

impl Describe for CommandOp {
    #[inline]
    fn describe(&self, _path: &CtxPath,_dest: &Path, _mode: DescribeMode) -> Pretty {
        sjoin3("executing command '", &self.command, "'")
    }
}

impl<'lua> Describe for FunctionOp<'lua> {
    #[inline]
    fn describe(&self, _path: &CtxPath, _dest: &Path, _mode: DescribeMode) -> Pretty {
        pretty("running lua function")
    }
}
