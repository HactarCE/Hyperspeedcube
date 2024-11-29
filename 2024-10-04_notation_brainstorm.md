# Notation

## Separators

()      grouping (can be multiplied)
[A,B]   commutator
[A:B]   conjugate
space   between moves

/       squan notation

## Prefixes

number  layer mask (+w means outer-block layers)
{..}    layer mask

## Substitutions

M E S     slice moves
lowercase inner layer (4x4)

## Suffixes

w         wide moves
1, 2, 3   multiplier
'         inverse
+, ++     jumbling (clockwise), or megaminx scrambling
-, --     jumbling (counterclockwise), or megaminx scrambling





- wide_move_suffix: bool (per axis)
- plus_minus_counting: bool (per twist)

## Axis names



## Twist names

concat('I', unordered('R', 'U', 'F'))

{ 'set', {'set', 'A', 'B'}, {'set', 'C', 'D'}}

{
  'any',

}

{
  'seq',
}

{
  'set', t:transform(a2).name, t:transform(a3).name,
}
