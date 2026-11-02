#!/usr/bin/env python3
"""KH1 FM enemy AI script reader (M11, `docs/recom-m11-enemies.md`): a linear listing of a `.bd`
script (part 5 of an enemy's `.mdls`) with the stack ops folded into expressions.

usage: python3 -I tools/kh1-ai-script.py <xa_xx_NNNN.mdls | script.bd> <KH1 Steam exe> [start_hex] [--raw]

Reads only the player's own files: the script from the model, the syscall table (argument counts, which
return a value) from the KH1 Steam exe at 0x140529180 (16-byte entries: function, descriptor; low byte =
argument count, bit 30 = returns a value), and table 1 (mode 1, 0x1405752D0) from the pointer array at
0x140529FA0. Nothing of the game is in this file.

The VM (`0x1402CC620`, Checked (decomp)): 16-bit words, low nibble = op, bits 4..5 = sub, 6..7 = mode
(0 sp frame, 1 wp work area, 2 hp = *sp[0] + off, 3 pc-relative), high byte = imm.
  op 0 misc: imm 0 yield (the rest of the frame), 3 return, 8 end the thread, 4 drop, 7 store (pop value,
       pop address), 12 dup, 13 not, 1/2 abs, 5/6 int, 9/10 negate, 11 not-bits.
  op 1 binary (sub 0 int, 1 float): imm 0 +, 1 −, 2 ×, 3 ÷, 4 %, 5 &, 6 |, 7 ^, 8 <<, 9 >>, 10 &&, 11 ||;
       "push a; push b; op" is a OP b.
  op 2 push: sub 0 int / 1 float (+4 bytes), 2 address (`pusha`, mode 3: pc + 2 + off), 3 load imm words.
  op 3 pop to address; op 4 jmp, 5 jz, 6 jnz (pc + 4 + off × 2); op 7 compare with 0 (imm 0 <, 1 <=,
  2 ==, 3 !=, 4 >=, 5 >); op 8 call (pc + 4 + off × 2; imm = the callee's frame words; the callee pops
  its arguments, the last pushed first); op 9 add imm × 4; op 10 load imm words; op 11 syscall imm of
  table mode (args popped, the last pushed is the last argument).
A handler (`sys 33`) or a behaviour pointer is pushed as an integer: its code offset is the value × 2.
"""
import struct
import sys

NAMES = {1: 'debug', 4: 'party', 5: 'step', 6: 'len', 7: 'vadd', 8: 'vsub', 9: 'norm', 10: 'rotdeg',
         11: 'face_now', 12: 'mplay', 13: 'mreq', 14: 'mqueue', 20: 'thread', 22: 'hitbit', 23: 'attack_new',
         25: 'randf', 33: 'handlers', 34: 'restart', 36: 'yaw_target', 40: 'move_toward', 43: 'mlen',
         44: 'damage', 45: 'stats', 48: 'search', 49: 'target', 50: 'jump', 51: 'cur_motion', 75: 'effect',
         93: 'vscale', 94: 'child_object', 105: 'world', 114: 'obj_kind', 116: 'mask_set', 117: 'mask_clr',
         122: 'drops', 125: 'turn_rate', 132: 'spawn_type', 139: 'occluded', 172: 'motion_done?', 198: 'lv2',
         225: 'effect_h'}


def part(d, k):
    n = struct.unpack_from('<i', d, 0)[0]
    if not 0 < n <= 64:
        return d
    a, b = struct.unpack_from('<ii', d, 4 + 4 * k)
    return d[a:b]


def sys_table(exe):
    import pefile  # the player's exe only
    pe = pefile.PE(exe, fast_load=True)
    out = []
    # The VM's table pointers (`0x1402CC620` op 11: `param[4] + mode * 8`): the script context's
    # array at 0x140529FA0 holds table 0 (0x140529180) and table 1 (0x1405752D0). Checked (exe).
    tables = [struct.unpack('<Q', pe.get_data(0x529FA0 + 8 * m, 8))[0] - 0x140000000 for m in range(2)]
    for mode_at in tables:
        tab = []
        for k in range(256):
            desc = struct.unpack('<I', pe.get_data(mode_at + 16 * k + 8, 4))[0]
            fn = struct.unpack('<Q', pe.get_data(mode_at + 16 * k, 8))[0]
            if not (0x140000000 <= fn < 0x150000000) and fn != 0:
                break
            tab.append(desc)
        out.append(tab)
    return out


def ilen(w):
    op, sub = w & 0xF, (w >> 4) & 3
    if op == 2:
        return 6 if sub <= 1 else 4
    if op in (3, 4, 5, 6, 8):
        return 4
    return 2


def decode(b, start):
    pc, xs = start, {}
    while pc + 2 <= len(b):
        w = struct.unpack_from('<H', b, pc)[0]
        n = ilen(w)
        if pc + n > len(b):
            break
        o = None
        if n == 4:
            o = struct.unpack_from('<h', b, pc + 2)[0]
        elif n == 6:
            o = struct.unpack_from('<i', b, pc + 2)[0]
        xs[pc] = (pc, w, w & 0xF, (w >> 4) & 3, (w >> 6) & 3, w >> 8, o, n)
        pc += n
    return xs


def f32(i):
    return struct.unpack('<f', struct.pack('<i', i))[0]


M = ['sp', 'wp', 'hp', 'pc']


def listing(b, start, tables, raw=False):
    xs = decode(b, start)
    pcs = sorted(xs)
    targets = {x[0] + 4 + x[6] * 2 for x in xs.values() if x[2] in (4, 5, 6, 8)}
    funcs = {x[0] + 4 + x[6] * 2 for x in xs.values() if x[2] == 8}
    funcs |= {x[6] * 2 for x in xs.values() if x[2] == 2 and x[3] == 0 and x[6] and x[6] * 2 in xs and x[6] * 2 >= start}

    def nargs(f):
        n, pc = 0, f
        while pc in xs and xs[pc][2] == 3 and xs[pc][4] == 0:
            n, pc = n + 1, pc + 4
        return n

    memo = {}

    def sysd(x):
        t = tables[x[4]] if x[4] < len(tables) else []
        d = t[x[5]] if x[5] < len(t) else 0
        return d & 0xFF, bool(d & 0x40000000)

    def rets(f):
        if f in memo:
            return memo[f]
        memo[f] = False
        nxt = [g for g in sorted(funcs) if g > f]
        end = nxt[0] if nxt else 1 << 30
        prev, r = None, False
        for pc in pcs:
            if pc < f:
                continue
            if pc >= end:
                break
            x = xs[pc]
            if x[2] == 0 and x[5] == 3 and prev is not None:
                p = prev
                if p[2] in (1, 2, 7, 9, 10) or (p[2] == 0 and p[5] in (1, 2, 5, 6, 9, 10, 11, 12, 13)):
                    r = True
                if p[2] == 11 and sysd(p)[1]:
                    r = True
                if p[2] == 8 and rets(p[0] + 4 + p[6] * 2):
                    r = True
            prev = x
        memo[f] = r
        return r

    out, st = [], []

    def pop():
        return st.pop() if st else '<?>'

    for pc in pcs:
        x = xs[pc]
        op, sub, mode, imm, o, n = x[2:8]
        if pc in funcs or pc in targets:
            for e in st:
                out.append(f'        (stack) {e}')
            st = []
            out.append(f'\nfunc_{pc:x}({nargs(pc)} args{", ->" if rets(pc) else ""}):' if pc in funcs else f'L_{pc:x}:')

        def emit(t):
            out.append(f'  {pc:5x}: {t}')
        if raw:
            emit(f'{x[1]:04x} op{op} sub{sub} mode{mode} imm{imm} {o}')
            continue
        if op == 0:
            if imm == 0:
                emit('yield')
            elif imm == 3:
                emit('return' + (f' {pop()}' if st else ''))
            elif imm == 8:
                emit('end_thread')
            elif imm == 4:
                emit(f'drop {pop()}')
            elif imm == 7:
                v = pop()
                a = pop()
                emit(f'*{a} = {v}')
            elif imm == 12:
                v = pop()
                st += [v, v]
            elif imm == 13:
                st.append(f'!{pop()}')
            elif imm in (1, 2):
                st.append(f'abs({pop()})')
            elif imm in (5, 6):
                st.append(f'int({pop()})')
            elif imm in (9, 10, 11):
                st.append(f'-{pop()}')
            else:
                emit(f'misc{imm}')
        elif op == 1:
            r, l = pop(), pop()
            ops = ['+', '-', '*', '/', '%', '&', '|', '^', '<<', '>>', '&&', '||']
            st.append(f'({l} {ops[imm] if imm < len(ops) else "?"}{"f" if sub == 1 else ""} {r})')
        elif op == 2:
            if sub <= 1:
                if sub == 1:
                    st.append(f'{f32(o):g}')
                else:
                    st.append(f'{o}' + (f' /*x2={o * 2:#x}*/' if o and o * 2 in xs and o > 256 else '') + (f' /*{o:#x}*/' if o > 0xFFFF else ''))
            elif sub == 2:
                st.append(f'&pc:{pc + 2 + o:#x}' if mode == 3 else f'&{M[mode]}[{o:#x}]')
            else:
                st.append(f'{M[mode]}[{o:#x}]' + (f'.{imm}w' if imm != 1 else ''))
        elif op == 3:
            emit(f'{M[mode]}[{o:#x}] = {pop()}')
        elif op == 4:
            emit(f'goto {pc + 4 + o * 2:#x}')
        elif op == 5:
            emit(f'if !{pop()} goto {pc + 4 + o * 2:#x}')
        elif op == 6:
            emit(f'if {pop()} goto {pc + 4 + o * 2:#x}')
        elif op == 7:
            c = ['<0', '<=0', '==0', '!=0', '>=0', '>0'][imm] if imm < 6 else '?'
            st.append(f'({pop()} {c})')
        elif op == 8:
            t = pc + 4 + o * 2
            args = [pop() for _ in range(nargs(t))][::-1]
            e = f'func_{t:x}({", ".join(args)})'
            if rets(t):
                st.append(e)
            else:
                emit(e)
        elif op == 9:
            st.append(f'({pop()}+{imm * 4})')
        elif op == 10:
            st.append(f'*{pop()}' + (f'.{imm}w' if imm != 1 else ''))
        elif op == 11:
            a, r = sysd(x)
            args = [pop() for _ in range(a)][::-1]
            nm = NAMES.get(imm, '') if mode == 0 else ''
            e = f'sys{mode}.{imm}{"_" + nm if nm else ""}({", ".join(args)})'
            if r:
                st.append(e)
            else:
                emit(e)
        else:
            emit(f'?{x[1]:#x}')
    return '\n'.join(out)


def first_code(b):
    """The lowest code offset: the entry jump (+0) and the pc-relative pushes' targets past the data."""
    w, j = struct.unpack_from('<Hh', b, 0)
    return 4 + j * 2


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return 1
    d = open(sys.argv[1], 'rb').read()
    b = part(d, 5) if sys.argv[1].endswith('.mdls') else d
    tables = sys_table(sys.argv[2])
    start = int(sys.argv[3], 16) if len(sys.argv) > 3 and not sys.argv[3].startswith('--') else None
    print(f'; {b[4:0x30].split(bytes(1))[0].decode("latin1")}: {len(b):#x} bytes, entry {first_code(b):#x}')
    if start is None:
        print('; give the first code offset (after the data: stat records, attack rows, drop tables); '
              'the entry routine is at the offset above')
        start = first_code(b)
    print(listing(b, start, tables, raw='--raw' in sys.argv))
    return 0


if __name__ == '__main__':
    sys.exit(main())
