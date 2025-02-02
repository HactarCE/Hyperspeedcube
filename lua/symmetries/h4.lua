function _120cell(scale, basis)
  return {
    name = "120-Cell",
    face_colors = '120_cell',
    sym = cd('h4', basis),
    iter_poles = function(self, prefix)
      return self.sym:orbit(self.sym.ooox.unit * (scale or 1)):named({
        RI = {4, 'FU'},
        RF = {4, 'LY'},
        RU = {4, 'FL'},
        RR = {4, 'OPB'},
        RL = {4, 'FBL'},
        RDR = {4, 'OPR'},
        RDL = {4, 'LDL'},
        RBR = {1, 'BPL'},
        RBL = {1, 'BL'},
        RPR = {4, 'OPD'},
        RPL = {4, 'FBR'},
        RPD = {4, 'FX'},
        RPB = {4, 'FR'},
        RX = {1, 'BPD'},
        RY = {4, 'LF'},

        LI = {3, 'LPB'},
        LF = {3, 'DL'},
        LU = {2, 'UPL'},
        LR = {2, 'UY'},
        LL = {3, 'OX'},
        LDR = {3, 'DU'},
        LDL = {3, 'DF'},
        LBR = {2, 'FPR'},
        LBL = {2, 'FR'},
        LPR = {2, 'FPB'},
        LPL = {2, 'FU'},
        LPD = {2, 'FBL'},
        LPB = {2, 'FI'},
        LX = {3, 'BF'},
        LY = {1, 'LPL'},

        UI = {3, 'BBR'},
        UF = {3, 'RDL'},
        UU = {3, 'RDR'},
        UR = {4, 'UPB'},
        UL = {3, 'RF'},
        UDR = {4, 'UPD'},
        UDL = {3, 'BX'},
        UBR = {3, 'BPB'},
        UBL = {3, 'DY'},
        UPR = {3, 'BI'},
        UPL = {3, 'DPL'},
        UPD = {3, 'BR'},
        UPB = {3, 'BPR'},
        UX = {4, 'ODR'},
        UY = {3, 'DBL'},

        DI = {2, 'DPR'},
        DF = {2, 'FL'},
        DU = {2, 'FPL'},
        DR = {1, 'LPB'},
        DL = {2, 'FDL'},
        DDR = {1, 'OX'},
        DDL = {2, 'RX'},
        DBR = {1, 'IY'},
        DBL = {4, 'DU'},
        DPR = {1, 'BDR'},
        DPL = {4, 'DF'},
        DPD = {1, 'BPR'},
        DPB = {1, 'BR'},
        DX = {1, 'LPR'},
        DY = {4, 'OBR'},

        FI = {1, 'LU'},
        FF = {1, 'BY'},
        FU = {3, 'DDR'},
        FR = {3, 'DPD'},
        FL = {1, 'LBL'},
        FDR = {3, 'RU'},
        FDL = {1, 'BDL'},
        FBR = {3, 'DI'},
        FBL = {3, 'DR'},
        FPR = {3, 'DPB'},
        FPL = {1, 'LBR'},
        FPD = {3, 'FY'},
        FPB = {1, 'LR'},
        FX = {3, 'DDL'},
        FY = {4, 'FPL'},

        BI = {4, 'LBL'},
        BF = {4, 'LBR'},
        BU = {4, 'LI'},
        BR = {2, 'RBL'},
        BL = {2, 'UF'},
        BDR = {2, 'FDR'},
        BDL = {2, 'UL'},
        BBR = {4, 'LPL'},
        BBL = {4, 'LL'},
        BPR = {2, 'RBR'},
        BPL = {2, 'UU'},
        BPD = {2, 'OY'},
        BPB = {4, 'OR'},
        BX = {4, 'LPD'},
        BY = {2, 'UBL'},

        OI = {},
        OF = {2, 'ODL'},
        OU = {2, 'OPL'},
        OR = {2, 'OPD'},
        OL = {4, 'OI'},
        ODR = {3, 'OU'},
        ODL = {3, 'OL'},
        OBR = {2, 'OPB'},
        OBL = {1, 'OU'},
        OPR = {3, 'OBR'},
        OPL = {1, 'OF'},
        OPD = {3, 'OBL'},
        OPB = {1, 'OR'},
        OX = {2, 'FF'},
        OY = {3, 'RR'},

        II = {4, 'IL'},
        IF = {1, 'IPL'},
        IU = {3, 'IDR'},
        IR = {1, 'IPB'},
        IL = {3, 'IDL'},
        IDR = {1, 'IPD'},
        IDL = {2, 'IF'},
        IBR = {3, 'IPR'},
        IBL = {3, 'IPD'},
        IPR = {4, 'LDR'},
        IPL = {2, 'IU'},
        IPD = {2, 'IR'},
        IPB = {2, 'IBR'},
        IX = {4, 'FPR'},
        IY = {2, 'FPD'},
      })
    end
  }
end

color_systems:add{
  id = '120_cell',
  name = "120-Cell",

  colors = {
    { name = 'RI',  display = 'Right ~ In',         default = "Rainbow [0/0]" },
    { name = 'RF',  display = 'Right ~ Up',         default = "Rainbow [0/0]" },
    { name = 'RU',  display = 'Right ~ Front',      default = "Rainbow [0/0]" },
    { name = 'RR',  display = 'Right ~ Right',      default = "Rainbow [0/0]" },
    { name = 'RL',  display = 'Right ~ Left',       default = "Rainbow [0/0]" },
    { name = 'RDR', display = 'Right ~ Back-right', default = "Rainbow [0/0]" },
    { name = 'RDL', display = 'Right ~ Back-left',  default = "Rainbow [0/0]" },
    { name = 'RBR', display = 'Right ~ Down-right', default = "Rainbow [0/0]" },
    { name = 'RBL', display = 'Right ~ Down-left',  default = "Rainbow [0/0]" },
    { name = 'RPR', display = 'Right ~ Para-right', default = "Rainbow [0/0]" },
    { name = 'RPL', display = 'Right ~ Para-left',  default = "Rainbow [0/0]" },
    { name = 'RPD', display = 'Right ~ Para-back',  default = "Rainbow [0/0]" },
    { name = 'RPB', display = 'Right ~ Para-down',  default = "Rainbow [0/0]" },
    { name = 'RX',  display = 'Right ~ X',          default = "Rainbow [0/0]" },
    { name = 'RY',  display = 'Right ~ Y',          default = "Rainbow [0/0]" },

    { name = 'LI',  display = 'Left ~ In',          default = "Rainbow [0/0]" },
    { name = 'LF',  display = 'Left ~ Up',          default = "Rainbow [0/0]" },
    { name = 'LU',  display = 'Left ~ Front',       default = "Rainbow [0/0]" },
    { name = 'LR',  display = 'Left ~ Right',       default = "Rainbow [0/0]" },
    { name = 'LL',  display = 'Left ~ Left',        default = "Rainbow [0/0]" },
    { name = 'LDR', display = 'Left ~ Back-right',  default = "Rainbow [0/0]" },
    { name = 'LDL', display = 'Left ~ Back-left',   default = "Rainbow [0/0]" },
    { name = 'LBR', display = 'Left ~ Down-right',  default = "Rainbow [0/0]" },
    { name = 'LBL', display = 'Left ~ Down-left',   default = "Rainbow [0/0]" },
    { name = 'LPR', display = 'Left ~ Para-right',  default = "Rainbow [0/0]" },
    { name = 'LPL', display = 'Left ~ Para-left',   default = "Rainbow [0/0]" },
    { name = 'LPD', display = 'Left ~ Para-back',   default = "Rainbow [0/0]" },
    { name = 'LPB', display = 'Left ~ Para-down',   default = "Rainbow [0/0]" },
    { name = 'LX',  display = 'Left ~ X',           default = "Rainbow [0/0]" },
    { name = 'LY',  display = 'Left ~ Y',           default = "Rainbow [0/0]" },

    { name = 'UI',  display = 'Up ~ In',            default = "Rainbow [0/0]" },
    { name = 'UF',  display = 'Up ~ Up',            default = "Rainbow [0/0]" },
    { name = 'UU',  display = 'Up ~ Front',         default = "Rainbow [0/0]" },
    { name = 'UR',  display = 'Up ~ Right',         default = "Rainbow [0/0]" },
    { name = 'UL',  display = 'Up ~ Left',          default = "Rainbow [0/0]" },
    { name = 'UDR', display = 'Up ~ Back-right',    default = "Rainbow [0/0]" },
    { name = 'UDL', display = 'Up ~ Back-left',     default = "Rainbow [0/0]" },
    { name = 'UBR', display = 'Up ~ Down-right',    default = "Rainbow [0/0]" },
    { name = 'UBL', display = 'Up ~ Down-left',     default = "Rainbow [0/0]" },
    { name = 'UPR', display = 'Up ~ Para-right',    default = "Rainbow [0/0]" },
    { name = 'UPL', display = 'Up ~ Para-left',     default = "Rainbow [0/0]" },
    { name = 'UPD', display = 'Up ~ Para-back',     default = "Rainbow [0/0]" },
    { name = 'UPB', display = 'Up ~ Para-down',     default = "Rainbow [0/0]" },
    { name = 'UX',  display = 'Up ~ X',             default = "Rainbow [0/0]" },
    { name = 'UY',  display = 'Up ~ Y',             default = "Rainbow [0/0]" },

    { name = 'DI',  display = 'Down ~ In',          default = "Rainbow [0/0]" },
    { name = 'DF',  display = 'Down ~ Up',          default = "Rainbow [0/0]" },
    { name = 'DU',  display = 'Down ~ Front',       default = "Rainbow [0/0]" },
    { name = 'DR',  display = 'Down ~ Right',       default = "Rainbow [0/0]" },
    { name = 'DL',  display = 'Down ~ Left',        default = "Rainbow [0/0]" },
    { name = 'DDR', display = 'Down ~ Back-right',  default = "Rainbow [0/0]" },
    { name = 'DDL', display = 'Down ~ Back-left',   default = "Rainbow [0/0]" },
    { name = 'DBR', display = 'Down ~ Down-right',  default = "Rainbow [0/0]" },
    { name = 'DBL', display = 'Down ~ Down-left',   default = "Rainbow [0/0]" },
    { name = 'DPR', display = 'Down ~ Para-right',  default = "Rainbow [0/0]" },
    { name = 'DPL', display = 'Down ~ Para-left',   default = "Rainbow [0/0]" },
    { name = 'DPD', display = 'Down ~ Para-back',   default = "Rainbow [0/0]" },
    { name = 'DPB', display = 'Down ~ Para-down',   default = "Rainbow [0/0]" },
    { name = 'DX',  display = 'Down ~ X',           default = "Rainbow [0/0]" },
    { name = 'DY',  display = 'Down ~ Y',           default = "Rainbow [0/0]" },

    { name = 'FI',  display = 'Front ~ In',         default = "Rainbow [0/0]" },
    { name = 'FF',  display = 'Front ~ Up',         default = "Rainbow [0/0]" },
    { name = 'FU',  display = 'Front ~ Front',      default = "Rainbow [0/0]" },
    { name = 'FR',  display = 'Front ~ Right',      default = "Rainbow [0/0]" },
    { name = 'FL',  display = 'Front ~ Left',       default = "Rainbow [0/0]" },
    { name = 'FDR', display = 'Front ~ Back-right', default = "Rainbow [0/0]" },
    { name = 'FDL', display = 'Front ~ Back-left',  default = "Rainbow [0/0]" },
    { name = 'FBR', display = 'Front ~ Down-right', default = "Rainbow [0/0]" },
    { name = 'FBL', display = 'Front ~ Down-left',  default = "Rainbow [0/0]" },
    { name = 'FPR', display = 'Front ~ Para-right', default = "Rainbow [0/0]" },
    { name = 'FPL', display = 'Front ~ Para-left',  default = "Rainbow [0/0]" },
    { name = 'FPD', display = 'Front ~ Para-back',  default = "Rainbow [0/0]" },
    { name = 'FPB', display = 'Front ~ Para-down',  default = "Rainbow [0/0]" },
    { name = 'FX',  display = 'Front ~ X',          default = "Rainbow [0/0]" },
    { name = 'FY',  display = 'Front ~ Y',          default = "Rainbow [0/0]" },

    { name = 'BI',  display = 'Back ~ In',          default = "Rainbow [0/0]" },
    { name = 'BF',  display = 'Back ~ Up',          default = "Rainbow [0/0]" },
    { name = 'BU',  display = 'Back ~ Front',       default = "Rainbow [0/0]" },
    { name = 'BR',  display = 'Back ~ Right',       default = "Rainbow [0/0]" },
    { name = 'BL',  display = 'Back ~ Left',        default = "Rainbow [0/0]" },
    { name = 'BDR', display = 'Back ~ Back-right',  default = "Rainbow [0/0]" },
    { name = 'BDL', display = 'Back ~ Back-left',   default = "Rainbow [0/0]" },
    { name = 'BBR', display = 'Back ~ Down-right',  default = "Rainbow [0/0]" },
    { name = 'BBL', display = 'Back ~ Down-left',   default = "Rainbow [0/0]" },
    { name = 'BPR', display = 'Back ~ Para-right',  default = "Rainbow [0/0]" },
    { name = 'BPL', display = 'Back ~ Para-left',   default = "Rainbow [0/0]" },
    { name = 'BPD', display = 'Back ~ Para-back',   default = "Rainbow [0/0]" },
    { name = 'BPB', display = 'Back ~ Para-down',   default = "Rainbow [0/0]" },
    { name = 'BX',  display = 'Back ~ X',           default = "Rainbow [0/0]" },
    { name = 'BY',  display = 'Back ~ Y',           default = "Rainbow [0/0]" },

    { name = 'OI',  display = 'Out ~ In',           default = "Rainbow [0/0]" },
    { name = 'OF',  display = 'Out ~ Up',           default = "Rainbow [0/0]" },
    { name = 'OU',  display = 'Out ~ Front',        default = "Rainbow [0/0]" },
    { name = 'OR',  display = 'Out ~ Right',        default = "Rainbow [0/0]" },
    { name = 'OL',  display = 'Out ~ Left',         default = "Rainbow [0/0]" },
    { name = 'ODR', display = 'Out ~ Back-right',   default = "Rainbow [0/0]" },
    { name = 'ODL', display = 'Out ~ Back-left',    default = "Rainbow [0/0]" },
    { name = 'OBR', display = 'Out ~ Down-right',   default = "Rainbow [0/0]" },
    { name = 'OBL', display = 'Out ~ Down-left',    default = "Rainbow [0/0]" },
    { name = 'OPR', display = 'Out ~ Para-right',   default = "Rainbow [0/0]" },
    { name = 'OPL', display = 'Out ~ Para-left',    default = "Rainbow [0/0]" },
    { name = 'OPD', display = 'Out ~ Para-back',    default = "Rainbow [0/0]" },
    { name = 'OPB', display = 'Out ~ Para-down',    default = "Rainbow [0/0]" },
    { name = 'OX',  display = 'Out ~ X',            default = "Rainbow [0/0]" },
    { name = 'OY',  display = 'Out ~ Y',            default = "Rainbow [0/0]" },

    { name = 'II',  display = 'In ~ In',            default = "Rainbow [0/0]" },
    { name = 'IF',  display = 'In ~ Up',            default = "Rainbow [0/0]" },
    { name = 'IU',  display = 'In ~ Front',         default = "Rainbow [0/0]" },
    { name = 'IR',  display = 'In ~ Right',         default = "Rainbow [0/0]" },
    { name = 'IL',  display = 'In ~ Left',          default = "Rainbow [0/0]" },
    { name = 'IDR', display = 'In ~ Back-right',    default = "Rainbow [0/0]" },
    { name = 'IDL', display = 'In ~ Back-left',     default = "Rainbow [0/0]" },
    { name = 'IBR', display = 'In ~ Down-right',    default = "Rainbow [0/0]" },
    { name = 'IBL', display = 'In ~ Down-left',     default = "Rainbow [0/0]" },
    { name = 'IPR', display = 'In ~ Para-right',    default = "Rainbow [0/0]" },
    { name = 'IPL', display = 'In ~ Para-left',     default = "Rainbow [0/0]" },
    { name = 'IPD', display = 'In ~ Para-back',     default = "Rainbow [0/0]" },
    { name = 'IPB', display = 'In ~ Para-down',     default = "Rainbow [0/0]" },
    { name = 'IX',  display = 'In ~ X',             default = "Rainbow [0/0]" },
    { name = 'IY',  display = 'In ~ Y',             default = "Rainbow [0/0]" },
  }
}

-- TODO: where to keep this?
local function _gen_120cell_names()
  local swirl = sym:thru(3,2,1,2,4,1,3,2,1,2,1,3,2,1,2,3,4,3,2,1,2,1,3,2,1,2,4,3,2,1,2,3,2,3).rev
  local transpose = sym:thru(1,2,4,1,3,2,1,2,1,3,2,1,2,3,4,3,2,1,2,1,3,2,1,2,4,3,2,1,2,3).rev

  local axes = self.axes:add(facet_poles)
  -- self.axes:autoname()
  -- self.axes[vec('x')].name = 'RI'
  -- self.axes[-vec('x')].name = 'LI'
  -- self.axes[vec('y')].name = 'UI'
  -- self.axes[-vec('y')].name = 'DI'
  -- self.axes[vec('z')].name = 'FI'
  -- self.axes[-vec('z')].name = 'BI'
  self.axes[sym.ooox].name = 'OI'
  sym:thru(2, 3, 4):transform(self.axes.OI).name = "OF"
  sym:thru(2, 1, 2, 3, 4):transform(self.axes.OI).name = "OU"

  local function name_hypercubic_cluster(Ob, beta_name)
    local Db = swirl:transform(Ob)
    local Lb = transpose:transform(Ob)
    local Bb = transpose:transform(Db)
    local Ib = self.axes[-Ob.vector]
    local Rb = self.axes[-Lb.vector]
    local Ub = self.axes[-Db.vector]
    local Fb = self.axes[-Bb.vector]
    Rb.name = 'R' .. beta_name
    Lb.name = 'L' .. beta_name
    Ub.name = 'U' .. beta_name
    Db.name = 'D' .. beta_name
    Fb.name = 'F' .. beta_name
    Bb.name = 'B' .. beta_name
    Ib.name = 'I' .. beta_name
    Ob.name = 'O' .. beta_name
  end

  local OI = self.axes.OI
  local OF = self.axes.OF
  local OU = self.axes.OU
  local OR = rot{fix = OF.vector ^ OI.vector, angle = pi*2/5}:transform(OU)

  name_hypercubic_cluster(self.axes.OI, 'I')

  local gen1 = refl(self.axes.RI.vector)
  local gen2 = refl(OU.vector - OR.vector)
  local gen3 = refl(OU.vector - OF.vector)
  for t, axis, name in symmetry({gen1, gen2, gen3}):orbit(self.axes.OF):named({
    F = {},
    U = {3, 'F'},
    R = {2, 'U'},
    L = {1, 'R'},
    DR = {2, 'L'},
    DL = {1, 'DR'},
    BR = {3, 'DR'},
    BL = {3, 'DL'},
    PR = {2, 'BL'},
    PL = {1, 'PR'},
    PD = {2, 'PL'},
    PB = {3, 'PD'},
  }) do
    name_hypercubic_cluster(axis, name)
  end

  local OX = sym:thru(2, 1, 2, 3, 4, 2, 1, 2, 3, 1, 2, 1, 2, 3, 4):transform(OI)
  name_hypercubic_cluster(OX, 'X')

  local OY = (refl() * refl(OI.vector)):transform(OX)
  name_hypercubic_cluster(OY, 'Y')

  print(OX.vector)
  print(OY.vector)
end
