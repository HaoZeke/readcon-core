# Phase A corpus lock for the Julia bindings.
# Valid fixtures match golden JSON (symbols, atom_ids, fixed, positions).
# Invalid fixtures fail to parse (rkr_read_first_frame returns C_NULL).

using Test
using ReadCon

const CORPUS = joinpath(dirname(dirname(dirname(@__DIR__))), "resources", "conformance")
const MANIFEST = joinpath(CORPUS, "manifest.toml")

function _unquote(raw::AbstractString)
    s = strip(raw)
    if length(s) >= 2 && startswith(s, '"') && endswith(s, '"')
        return s[2:end-1]
    end
    return String(s)
end

function parse_manifest(text::AbstractString)
    cases = Dict{String,Any}[]
    current = nothing
    for raw in split(text, '\n')
        line = strip(raw)
        isempty(line) && continue
        startswith(line, '#') && continue
        if line == "[[valid]]" || line == "[[invalid]]"
            current !== nothing && push!(cases, current)
            current = Dict{String,Any}(
                "kind" => line == "[[valid]]" ? "valid" : "invalid",
                "id" => "",
                "path" => "",
            )
            continue
        end
        current === nothing && continue
        eq = findfirst('=', line)
        eq === nothing && continue
        key = strip(line[1:prevind(line, eq)])
        val = strip(line[nextind(line, eq):end])
        if key == "id"
            current["id"] = _unquote(val)
        elseif key == "path"
            current["path"] = _unquote(val)
        end
    end
    current !== nothing && push!(cases, current)
    return cases
end

function _key_colon(text::AbstractString, key::AbstractString)
    needle = "\"$key\""
    i = findfirst(needle, text)
    i === nothing && error("golden missing $key")
    p = nextind(text, last(i))
    while p <= lastindex(text) && isspace(text[p])
        p = nextind(text, p)
    end
    (p > lastindex(text) || text[p] != ':') && error("expected : after $key")
    p = nextind(text, p)
    while p <= lastindex(text) && isspace(text[p])
        p = nextind(text, p)
    end
    return p
end

function _parse_json_string(text::AbstractString, p::Int)
    while p <= lastindex(text) && isspace(text[p])
        p = nextind(text, p)
    end
    text[p] == '"' || error("expected string")
    p = nextind(text, p)
    start = p
    while p <= lastindex(text) && text[p] != '"'
        p = nextind(text, p)
    end
    p > lastindex(text) && error("unterminated string")
    return String(text[start:prevind(text, p)]), nextind(text, p)
end

function _parse_json_int(text::AbstractString, p::Int)
    while p <= lastindex(text) && isspace(text[p])
        p = nextind(text, p)
    end
    stop = p
    if stop <= lastindex(text) && text[stop] == '-'
        stop = nextind(text, stop)
    end
    while stop <= lastindex(text) && isdigit(text[stop])
        stop = nextind(text, stop)
    end
    stop == p && error("expected integer")
    return parse(Int, text[p:prevind(text, stop)]), stop
end

function _array_blob(text::AbstractString, key::AbstractString)
    p = _key_colon(text, key)
    text[p] == '[' || error("expected array for $key")
    depth = 0
    start = p
    for j in p:lastindex(text)
        c = text[j]
        if c == '['
            depth += 1
        elseif c == ']'
            depth -= 1
            depth == 0 && return String(text[start:j])
        end
    end
    error("unclosed array $key")
end

function _skip_ws_comma(blob::AbstractString, p::Int)
    while p <= lastindex(blob)
        c = blob[p]
        if isspace(c) || c == ','
            p = nextind(blob, p)
        else
            break
        end
    end
    return p
end

function _after_open_bracket(blob::AbstractString)
    p = 1
    while p <= lastindex(blob) && isspace(blob[p])
        p = nextind(blob, p)
    end
    (p <= lastindex(blob) && blob[p] == '[') || error("expected [")
    return nextind(blob, p)
end

function parse_golden(text::AbstractString)
    p = _key_colon(text, "id")
    id, _ = _parse_json_string(text, p)
    p = _key_colon(text, "n_atoms")
    n_atoms, _ = _parse_json_int(text, p)
    p = _key_colon(text, "spec_version")
    spec_version, _ = _parse_json_int(text, p)

    symbols = String[]
    blob = _array_blob(text, "symbols")
    q = _after_open_bracket(blob)
    for _ in 1:n_atoms
        q = _skip_ws_comma(blob, q)
        s, q = _parse_json_string(blob, q)
        push!(symbols, s)
    end

    atom_ids = Int[]
    blob = _array_blob(text, "atom_ids")
    q = _after_open_bracket(blob)
    for _ in 1:n_atoms
        q = _skip_ws_comma(blob, q)
        v, q = _parse_json_int(blob, q)
        push!(atom_ids, v)
    end

    fixed = Vector{NTuple{3,Bool}}()
    blob = _array_blob(text, "fixed")
    q = _after_open_bracket(blob)
    for _ in 1:n_atoms
        q = _skip_ws_comma(blob, q)
        blob[q] == '[' || error("expected [fx,fy,fz]")
        q = nextind(blob, q)
        bits = Bool[]
        for _j in 1:3
            q = _skip_ws_comma(blob, q)
            if startswith(blob[q:end], "true")
                push!(bits, true)
                q += 4
            elseif startswith(blob[q:end], "false")
                push!(bits, false)
                q += 5
            else
                error("expected bool in fixed")
            end
        end
        q = _skip_ws_comma(blob, q)
        blob[q] == ']' || error("expected ] after fixed row")
        q = nextind(blob, q)
        push!(fixed, (bits[1], bits[2], bits[3]))
    end

    positions = Vector{NTuple{3,Float64}}()
    blob = _array_blob(text, "positions")
    q = _after_open_bracket(blob)
    for _ in 1:n_atoms
        q = _skip_ws_comma(blob, q)
        blob[q] == '[' || error("expected [x,y,z]")
        q = nextind(blob, q)
        nums = Float64[]
        for _j in 1:3
            q = _skip_ws_comma(blob, q)
            stop = q
            if stop <= lastindex(blob) && (blob[stop] == '-' || blob[stop] == '+')
                stop = nextind(blob, stop)
            end
            while stop <= lastindex(blob) && (isdigit(blob[stop]) || blob[stop] in ('.', 'e', 'E', '+', '-'))
                stop = nextind(blob, stop)
            end
            push!(nums, parse(Float64, blob[q:prevind(blob, stop)]))
            q = stop
        end
        q = _skip_ws_comma(blob, q)
        blob[q] == ']' || error("expected ] after position row")
        q = nextind(blob, q)
        push!(positions, (nums[1], nums[2], nums[3]))
    end

    return Dict{String,Any}(
        "id" => id,
        "n_atoms" => n_atoms,
        "spec_version" => spec_version,
        "symbols" => symbols,
        "atom_ids" => atom_ids,
        "fixed" => fixed,
        "positions" => positions,
    )
end

function z_to_symbol(z::Integer)
    p = ccall(ReadCon._lib_symbol(:rkr_z_to_symbol), Cstring, (UInt64,), UInt64(z))
    p == C_NULL && return "X"
    return unsafe_string(p)
end

function read_first_ptr(path::AbstractString)
    return ccall(ReadCon._lib_symbol(:rkr_read_first_frame), Ptr{Cvoid}, (Cstring,), path)
end

@testset "conformance goldens" begin
    @test isfile(MANIFEST)
    cases = parse_manifest(read(MANIFEST, String))
    valids = [c for c in cases if c["kind"] == "valid"]
    invalids = [c for c in cases if c["kind"] == "invalid"]
    @test !isempty(valids) && !isempty(invalids)
    on_disk = Set(basename(p) for p in readdir(joinpath(CORPUS, "golden"); join=true) if endswith(p, ".json"))
    @test on_disk == Set(c["id"] * ".json" for c in valids)
    for case in invalids
        @test !isfile(joinpath(CORPUS, "golden", case["id"] * ".json"))
    end

    for case in cases
        fixture = joinpath(CORPUS, case["path"])
        @test isfile(fixture)
        if case["kind"] == "invalid"
            ptr = read_first_ptr(fixture)
            @test ptr == C_NULL
            continue
        end
        frames = read_con(fixture)
        @test length(frames) == 1
        frame = frames[1]
        golden = parse_golden(read(joinpath(CORPUS, "golden", case["id"] * ".json"), String))
        @test golden["id"] == case["id"]
        @test golden["n_atoms"] == length(frame.atoms)
        @test golden["spec_version"] == Int(frame.spec_version)
        atoms = frame.atoms
        @test [z_to_symbol(a.atomic_number) for a in atoms] == golden["symbols"]
        @test [Int(a.atom_id) for a in atoms] == golden["atom_ids"]
        @test [a.fixed for a in atoms] == golden["fixed"]
        for (atom, want) in zip(atoms, golden["positions"])
            @test atom.x == want[1]
            @test atom.y == want[2]
            @test atom.z == want[3]
        end
    end
end
