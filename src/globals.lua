-- name 'test'
function name(value) pkg:name(value) end

-- dep 'path'
-- dep {'path', ...}
-- dep 'path1' 'path2' ...
-- dep {'path1', 'path2', ...} { ... } ...
function dep(...)
    pkg:dep(...)
    return dep
end

-- file 'a.txt'
-- file {'b.txt'}
-- file {'c.txt', 'd.txt'}
-- file {'e.txt', 'f.txt', type = 'Copy'}
-- file {'g.txt', type = 'Copy'}
function file(arg)
    local src, dest, link_type
    if type(arg) == "string" then
        src = arg
        dest = nil
        link_type = nil
    elseif type(arg) == "table" then
        src = arg[1] or error('file src path was not provided')
        dest = arg[2]
        link_type = arg.type
    else
        error('invalid file directive')
    end

    local link_type = link_type or "Link"
    pkg:file(src, dest, link_type)
end

-- template {'d.hbs', 'j.txt', engine = 'handlebars', vars = {}}
-- template {'d.tmpl', 'k.txt', engine = 'liquid', vars = {}}
function template(arg)
    if type(arg) == "table" then
        local engine = arg.engine or error('template engine was not provided')
        if engine == 'hbs' then
            hbs(arg)
        elseif engine == 'liquid' then
            liquid(arg)
        else
            error('template engine must be hbs or liquid')
        end
    else
        error('template arg must be a table')
    end
end

-- hbs {'b.hbs', 'h.txt', vars = {}}
function hbs(arg)
    src = arg[1] or error('template src was not provided')
    dest = arg[2] or error('template dest was not provided')
    vars = arg.vars
    partials = arg.partials or {}

    pkg:hbs(src, dest, vars, partials)
end

-- liquid {'b.tmpl', 'i.txt', vars = {}}
function liquid(arg)
    src = arg[1] or error('template src was not provided')
    dest = arg[2] or error('template dest was not provided')
    vars = arg.vars

    pkg:liquid(src, dest, vars)
end

-- empty 'l.txt'
-- empty {'m.txt'}
function empty(arg)
    if type(arg) == "string" then
        pkg:empty(arg)
    elseif type(arg) == "table" then
        local path = arg[1] or error('empty dest was not provided')
        pkg:empty(path)
    else
        error('empty dest must be a string or table')
    end
end

-- string {'n.txt', 'contents'}
function str(arg)
    if type(arg) == "table" then
        local dest = arg[1] or error('str dest was not provided')
        local contents = arg[2] or error('str contents was not provided')
        pkg:str(dest, contents)
    else
        error('string arg must be a table')
    end
end

-- yaml {'o.txt', {}}
-- yaml {'p.txt', {}, header = '# header'}
function yaml(arg)
    if type(arg) == "table" then
        local dest = arg[1] or error('yaml dest was not provided')
        local values = arg[2] or error('yaml values were not provided')
        local header = arg.header
        pkg:yaml(dest, values, header)
    else
        error('string arg must be a table')
    end
end

-- toml {'q.txt', {}}
-- toml {'r.txt', {}, header = '# header'}
function toml(arg)
    if type(arg) == "table" then
        local dest = arg[1] or error('toml dest was not provided')
        local values = arg[2] or error('toml values were not provided')
        local header = arg.header
        pkg:toml(dest, values, header)
    else
        error('string arg must be a table')
    end
end

-- json {'s.txt', {}}
function json(arg)
    if type(arg) == "table" then
        local dest = arg[1] or error('toml dest was not provided')
        local values = arg[2] or error('toml values were not provided')
        pkg:json(dest, values)
    else
        error('string arg must be a table')
    end
end
