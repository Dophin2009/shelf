pkg = {}

dependency = function(path)
    if pkg.dependencies == nil then pkg.dependencies = {} end
    table.insert(pkg.dependencies, path)
end

file = function(src, dest, link_type, replace_files, replace_dirs)
    if pkg.files == nil then pkg.files = {} end
    table.insert(pkg.files, {
        src = src,
        dest = dest,
        link_type = link_type,
        replace_files = replace_files,
        replace_dirs = replace_dirs
    })
end

template = function(src, dest, engine, replace_files, replace_dirs)
    if pkg.templates == nil then pkg.templates = {} end
    table.insert(pkg.templates, {
        src = src,
        dest = dest,
        engine = engine,
        replace_files = replace_files,
        replace_dirs = replace_dirs
    })
end

