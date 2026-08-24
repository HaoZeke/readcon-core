/*
 * Phase A corpus lock for the C ABI.
 *
 * Reads resources/conformance/manifest.toml. Valid fixtures must match the
 * sibling golden JSON (symbols, atom_ids, fixed, positions). Invalid
 * fixtures must fail to parse (rkr_read_first_frame returns NULL).
 */
#include "readcon-core.h"

#include <ctype.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAX_CASES 32
#define MAX_ATOMS 64
#define MAX_SYM 8
#define MAX_ID 128
#define MAX_PATH 256
#define MAX_KIND 16

typedef struct {
    char kind[MAX_KIND];
    char id[MAX_ID];
    char path[MAX_PATH];
} Case;

typedef struct {
    char id[MAX_ID];
    int n_atoms;
    int spec_version;
    char symbols[MAX_ATOMS][MAX_SYM];
    uint64_t atom_ids[MAX_ATOMS];
    bool fixed[MAX_ATOMS][3];
    double positions[MAX_ATOMS][3];
} Golden;

static int g_fail;

static void failf(const char *fmt, const char *a) {
    fprintf(stderr, "FAIL: ");
    fprintf(stderr, fmt, a);
    fputc('\n', stderr);
    g_fail++;
}

static int file_exists(const char *path) {
    FILE *f = fopen(path, "rb");
    if (f == NULL) {
        return 0;
    }
    fclose(f);
    return 1;
}

static char *read_file(const char *path, size_t *out_len) {
    FILE *f = fopen(path, "rb");
    if (f == NULL) {
        return NULL;
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        fclose(f);
        return NULL;
    }
    long n = ftell(f);
    if (n < 0) {
        fclose(f);
        return NULL;
    }
    if (fseek(f, 0, SEEK_SET) != 0) {
        fclose(f);
        return NULL;
    }
    char *buf = (char *)malloc((size_t)n + 1);
    if (buf == NULL) {
        fclose(f);
        return NULL;
    }
    size_t got = fread(buf, 1, (size_t)n, f);
    fclose(f);
    buf[got] = '\0';
    if (out_len != NULL) {
        *out_len = got;
    }
    return buf;
}

static void unquote(const char *raw, char *out, size_t n) {
    const char *s = raw;
    while (*s && isspace((unsigned char)*s)) {
        s++;
    }
    size_t len = strlen(s);
    while (len > 0 && isspace((unsigned char)s[len - 1])) {
        len--;
    }
    if (len >= 2 && s[0] == '"' && s[len - 1] == '"') {
        s++;
        len -= 2;
    }
    if (len >= n) {
        len = n - 1;
    }
    memcpy(out, s, len);
    out[len] = '\0';
}

static int parse_manifest(const char *text, Case *cases, int cap) {
    int n = 0;
    Case *cur = NULL;
    const char *p = text;
    char line[512];
    while (*p) {
        size_t i = 0;
        while (*p && *p != '\n' && i + 1 < sizeof line) {
            line[i++] = *p++;
        }
        if (*p == '\n') {
            p++;
        }
        line[i] = '\0';
        char *s = line;
        while (*s && isspace((unsigned char)*s)) {
            s++;
        }
        if (*s == '\0' || *s == '#') {
            continue;
        }
        if (strcmp(s, "[[valid]]") == 0 || strcmp(s, "[[invalid]]") == 0) {
            if (n >= cap) {
                fprintf(stderr, "FAIL: too many manifest cases\n");
                g_fail++;
                return n;
            }
            cur = &cases[n++];
            memset(cur, 0, sizeof *cur);
            strncpy(cur->kind, s[2] == 'v' ? "valid" : "invalid",
                    sizeof cur->kind - 1);
            continue;
        }
        if (cur == NULL) {
            continue;
        }
        char *eq = strchr(s, '=');
        if (eq == NULL) {
            continue;
        }
        *eq = '\0';
        char *key = s;
        char *val = eq + 1;
        while (*key && isspace((unsigned char)*key)) {
            key++;
        }
        size_t klen = strlen(key);
        while (klen > 0 && isspace((unsigned char)key[klen - 1])) {
            key[--klen] = '\0';
        }
        if (strcmp(key, "id") == 0) {
            unquote(val, cur->id, sizeof cur->id);
        } else if (strcmp(key, "path") == 0) {
            unquote(val, cur->path, sizeof cur->path);
        }
    }
    return n;
}

static const char *skip_ws(const char *p) {
    while (*p && isspace((unsigned char)*p)) {
        p++;
    }
    return p;
}

static const char *find_key(const char *json, const char *key) {
    char pat[160];
    if (snprintf(pat, sizeof pat, "\"%s\"", key) >= (int)sizeof pat) {
        return NULL;
    }
    const char *p = strstr(json, pat);
    if (p == NULL) {
        return NULL;
    }
    p += strlen(pat);
    p = skip_ws(p);
    if (*p != ':') {
        return NULL;
    }
    return skip_ws(p + 1);
}

static int parse_json_string(const char **pp, char *out, size_t n) {
    const char *p = skip_ws(*pp);
    if (*p != '"') {
        return -1;
    }
    p++;
    size_t i = 0;
    while (*p && *p != '"') {
        if (i + 1 < n) {
            out[i++] = *p;
        }
        p++;
    }
    if (*p != '"') {
        return -1;
    }
    out[i] = '\0';
    *pp = p + 1;
    return 0;
}

static int parse_json_int(const char **pp, int *out) {
    char *end = NULL;
    const char *p = skip_ws(*pp);
    long v = strtol(p, &end, 10);
    if (end == p) {
        return -1;
    }
    *out = (int)v;
    *pp = end;
    return 0;
}

static int parse_json_u64(const char **pp, uint64_t *out) {
    char *end = NULL;
    const char *p = skip_ws(*pp);
    unsigned long long v = strtoull(p, &end, 10);
    if (end == p) {
        return -1;
    }
    *out = (uint64_t)v;
    *pp = end;
    return 0;
}

static int parse_json_double(const char **pp, double *out) {
    char *end = NULL;
    const char *p = skip_ws(*pp);
    double v = strtod(p, &end);
    if (end == p) {
        return -1;
    }
    *out = v;
    *pp = end;
    return 0;
}

static int parse_json_bool(const char **pp, bool *out) {
    const char *p = skip_ws(*pp);
    if (strncmp(p, "true", 4) == 0) {
        *out = true;
        *pp = p + 4;
        return 0;
    }
    if (strncmp(p, "false", 5) == 0) {
        *out = false;
        *pp = p + 5;
        return 0;
    }
    return -1;
}

static int parse_golden(const char *json, Golden *g) {
    memset(g, 0, sizeof *g);
    const char *p = find_key(json, "id");
    if (p == NULL || parse_json_string(&p, g->id, sizeof g->id) != 0) {
        return -1;
    }
    p = find_key(json, "n_atoms");
    if (p == NULL || parse_json_int(&p, &g->n_atoms) != 0) {
        return -1;
    }
    p = find_key(json, "spec_version");
    if (p == NULL || parse_json_int(&p, &g->spec_version) != 0) {
        return -1;
    }
    if (g->n_atoms < 0 || g->n_atoms > MAX_ATOMS) {
        return -1;
    }

    p = find_key(json, "symbols");
    if (p == NULL) {
        return -1;
    }
    p = skip_ws(p);
    if (*p != '[') {
        return -1;
    }
    p++;
    for (int i = 0; i < g->n_atoms; i++) {
        p = skip_ws(p);
        if (i > 0) {
            if (*p != ',') {
                return -1;
            }
            p = skip_ws(p + 1);
        }
        if (parse_json_string(&p, g->symbols[i], MAX_SYM) != 0) {
            return -1;
        }
    }

    p = find_key(json, "atom_ids");
    if (p == NULL) {
        return -1;
    }
    p = skip_ws(p);
    if (*p != '[') {
        return -1;
    }
    p++;
    for (int i = 0; i < g->n_atoms; i++) {
        p = skip_ws(p);
        if (i > 0) {
            if (*p != ',') {
                return -1;
            }
            p = skip_ws(p + 1);
        }
        if (parse_json_u64(&p, &g->atom_ids[i]) != 0) {
            return -1;
        }
    }

    p = find_key(json, "fixed");
    if (p == NULL) {
        return -1;
    }
    p = skip_ws(p);
    if (*p != '[') {
        return -1;
    }
    p++;
    for (int i = 0; i < g->n_atoms; i++) {
        p = skip_ws(p);
        if (i > 0) {
            if (*p != ',') {
                return -1;
            }
            p = skip_ws(p + 1);
        }
        if (*p != '[') {
            return -1;
        }
        p++;
        for (int j = 0; j < 3; j++) {
            p = skip_ws(p);
            if (j > 0) {
                if (*p != ',') {
                    return -1;
                }
                p = skip_ws(p + 1);
            }
            if (parse_json_bool(&p, &g->fixed[i][j]) != 0) {
                return -1;
            }
        }
        p = skip_ws(p);
        if (*p != ']') {
            return -1;
        }
        p++;
    }

    p = find_key(json, "positions");
    if (p == NULL) {
        return -1;
    }
    p = skip_ws(p);
    if (*p != '[') {
        return -1;
    }
    p++;
    for (int i = 0; i < g->n_atoms; i++) {
        p = skip_ws(p);
        if (i > 0) {
            if (*p != ',') {
                return -1;
            }
            p = skip_ws(p + 1);
        }
        if (*p != '[') {
            return -1;
        }
        p++;
        for (int j = 0; j < 3; j++) {
            p = skip_ws(p);
            if (j > 0) {
                if (*p != ',') {
                    return -1;
                }
                p = skip_ws(p + 1);
            }
            if (parse_json_double(&p, &g->positions[i][j]) != 0) {
                return -1;
            }
        }
        p = skip_ws(p);
        if (*p != ']') {
            return -1;
        }
        p++;
    }
    return 0;
}

static int find_root(char *out, size_t n, int argc, char **argv) {
    const char *env = getenv("READCON_CORE_ROOT");
    if (env != NULL && env[0] != '\0') {
        snprintf(out, n, "%s", env);
        return 0;
    }
    if (argc > 1 && argv[1][0] != '\0') {
        snprintf(out, n, "%s", argv[1]);
        return 0;
    }
    static const char *cands[] = {".", "..", "../..", "../../..",
                                  "../../../..", NULL};
    char probe[1024];
    for (int i = 0; cands[i] != NULL; i++) {
        snprintf(probe, sizeof probe, "%s/resources/conformance/manifest.toml",
                 cands[i]);
        if (file_exists(probe)) {
            snprintf(out, n, "%s", cands[i]);
            return 0;
        }
    }
    return -1;
}

static void check_valid(const char *root, const Case *c) {
    char fixture[1024];
    char golden_path[1024];
    snprintf(fixture, sizeof fixture, "%s/resources/conformance/%s", root,
             c->path);
    snprintf(golden_path, sizeof golden_path,
             "%s/resources/conformance/golden/%s.json", root, c->id);
    if (!file_exists(golden_path)) {
        failf("%s: missing golden JSON", c->id);
        return;
    }
    char *gtext = read_file(golden_path, NULL);
    if (gtext == NULL) {
        failf("%s: cannot read golden", c->id);
        return;
    }
    Golden g;
    if (parse_golden(gtext, &g) != 0) {
        failf("%s: golden JSON parse failed", c->id);
        free(gtext);
        return;
    }
    free(gtext);
    if (strcmp(g.id, c->id) != 0) {
        failf("%s: golden id mismatch", c->id);
        return;
    }

    RKRConFrame *frame = rkr_read_first_frame(fixture);
    if (frame == NULL) {
        failf("%s: valid fixture failed to parse", c->id);
        return;
    }
    uint32_t spec = rkr_frame_spec_version(frame);
    CFrame *cf = rkr_frame_to_c_frame(frame);
    if (cf == NULL) {
        failf("%s: rkr_frame_to_c_frame failed", c->id);
        free_rkr_frame(frame);
        return;
    }
    if ((int)cf->num_atoms != g.n_atoms) {
        failf("%s: n_atoms mismatch", c->id);
    }
    if ((int)spec != g.spec_version) {
        failf("%s: spec_version mismatch", c->id);
    }
    int n = g.n_atoms;
    if ((int)cf->num_atoms < n) {
        n = (int)cf->num_atoms;
    }
    for (int i = 0; i < n; i++) {
        const CAtom *a = &cf->atoms[i];
        const char *sym = rkr_z_to_symbol(a->atomic_number);
        if (sym == NULL || strcmp(sym, g.symbols[i]) != 0) {
            failf("%s: symbol mismatch", c->id);
        }
        if (a->atom_id != g.atom_ids[i]) {
            failf("%s: atom_id mismatch", c->id);
        }
        if (a->fixed_x != g.fixed[i][0] || a->fixed_y != g.fixed[i][1] ||
            a->fixed_z != g.fixed[i][2]) {
            failf("%s: fixed mismatch", c->id);
        }
        if (a->x != g.positions[i][0] || a->y != g.positions[i][1] ||
            a->z != g.positions[i][2]) {
            failf("%s: position mismatch", c->id);
        }
    }
    free_c_frame(cf);
    free_rkr_frame(frame);
}

static void check_invalid(const char *root, const Case *c) {
    char fixture[1024];
    char golden_path[1024];
    snprintf(fixture, sizeof fixture, "%s/resources/conformance/%s", root,
             c->path);
    snprintf(golden_path, sizeof golden_path,
             "%s/resources/conformance/golden/%s.json", root, c->id);
    if (file_exists(golden_path)) {
        failf("%s: invalid case must not have a golden", c->id);
    }
    RKRConFrame *frame = rkr_read_first_frame(fixture);
    if (frame != NULL) {
        failf("%s: invalid fixture parsed", c->id);
        free_rkr_frame(frame);
    }
}

int main(int argc, char **argv) {
    char root[1024];
    if (find_root(root, sizeof root, argc, argv) != 0) {
        fprintf(stderr,
                "FAIL: cannot find repo root; set READCON_CORE_ROOT\n");
        return 1;
    }
    char man_path[1100];
    snprintf(man_path, sizeof man_path,
             "%s/resources/conformance/manifest.toml", root);
    char *text = read_file(man_path, NULL);
    if (text == NULL) {
        fprintf(stderr, "FAIL: cannot read %s\n", man_path);
        return 1;
    }
    Case cases[MAX_CASES];
    int n = parse_manifest(text, cases, MAX_CASES);
    free(text);
    int nvalid = 0;
    int ninvalid = 0;
    for (int i = 0; i < n; i++) {
        if (strcmp(cases[i].kind, "valid") == 0) {
            nvalid++;
            check_valid(root, &cases[i]);
        } else {
            ninvalid++;
            check_invalid(root, &cases[i]);
        }
    }
    if (nvalid == 0 || ninvalid == 0) {
        fprintf(stderr, "FAIL: manifest has no valid or invalid cases\n");
        g_fail++;
    }
    if (g_fail != 0) {
        fprintf(stderr, "C conformance goldens: %d failure(s)\n", g_fail);
        return 1;
    }
    printf("C conformance goldens: %d valid, %d invalid OK\n", nvalid,
           ninvalid);
    return 0;
}
