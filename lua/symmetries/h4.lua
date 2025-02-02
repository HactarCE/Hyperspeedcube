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
    { name = 'RI',  display = 'Right ~ In' },
    { name = 'RF',  display = 'Right ~ Up' },
    { name = 'RU',  display = 'Right ~ Front' },
    { name = 'RR',  display = 'Right ~ Right' },
    { name = 'RL',  display = 'Right ~ Left' },
    { name = 'RDR', display = 'Right ~ Back-right' },
    { name = 'RDL', display = 'Right ~ Back-left' },
    { name = 'RBR', display = 'Right ~ Down-right' },
    { name = 'RBL', display = 'Right ~ Down-left' },
    { name = 'RPR', display = 'Right ~ Para-right' },
    { name = 'RPL', display = 'Right ~ Para-left' },
    { name = 'RPD', display = 'Right ~ Para-back' },
    { name = 'RPB', display = 'Right ~ Para-down' },
    { name = 'RX',  display = 'Right ~ X' },
    { name = 'RY',  display = 'Right ~ Y' },

    { name = 'LI',  display = 'Left ~ In' },
    { name = 'LF',  display = 'Left ~ Up' },
    { name = 'LU',  display = 'Left ~ Front' },
    { name = 'LR',  display = 'Left ~ Right' },
    { name = 'LL',  display = 'Left ~ Left' },
    { name = 'LDR', display = 'Left ~ Back-right' },
    { name = 'LDL', display = 'Left ~ Back-left' },
    { name = 'LBR', display = 'Left ~ Down-right' },
    { name = 'LBL', display = 'Left ~ Down-left' },
    { name = 'LPR', display = 'Left ~ Para-right' },
    { name = 'LPL', display = 'Left ~ Para-left' },
    { name = 'LPD', display = 'Left ~ Para-back' },
    { name = 'LPB', display = 'Left ~ Para-down' },
    { name = 'LX',  display = 'Left ~ X' },
    { name = 'LY',  display = 'Left ~ Y' },

    { name = 'UI',  display = 'Up ~ In' },
    { name = 'UF',  display = 'Up ~ Up' },
    { name = 'UU',  display = 'Up ~ Front' },
    { name = 'UR',  display = 'Up ~ Right' },
    { name = 'UL',  display = 'Up ~ Left' },
    { name = 'UDR', display = 'Up ~ Back-right' },
    { name = 'UDL', display = 'Up ~ Back-left' },
    { name = 'UBR', display = 'Up ~ Down-right' },
    { name = 'UBL', display = 'Up ~ Down-left' },
    { name = 'UPR', display = 'Up ~ Para-right' },
    { name = 'UPL', display = 'Up ~ Para-left' },
    { name = 'UPD', display = 'Up ~ Para-back' },
    { name = 'UPB', display = 'Up ~ Para-down' },
    { name = 'UX',  display = 'Up ~ X' },
    { name = 'UY',  display = 'Up ~ Y' },

    { name = 'DI',  display = 'Down ~ In' },
    { name = 'DF',  display = 'Down ~ Up' },
    { name = 'DU',  display = 'Down ~ Front' },
    { name = 'DR',  display = 'Down ~ Right' },
    { name = 'DL',  display = 'Down ~ Left' },
    { name = 'DDR', display = 'Down ~ Back-right' },
    { name = 'DDL', display = 'Down ~ Back-left' },
    { name = 'DBR', display = 'Down ~ Down-right' },
    { name = 'DBL', display = 'Down ~ Down-left' },
    { name = 'DPR', display = 'Down ~ Para-right' },
    { name = 'DPL', display = 'Down ~ Para-left' },
    { name = 'DPD', display = 'Down ~ Para-back' },
    { name = 'DPB', display = 'Down ~ Para-down' },
    { name = 'DX',  display = 'Down ~ X' },
    { name = 'DY',  display = 'Down ~ Y' },

    { name = 'FI',  display = 'Front ~ In' },
    { name = 'FF',  display = 'Front ~ Up' },
    { name = 'FU',  display = 'Front ~ Front' },
    { name = 'FR',  display = 'Front ~ Right' },
    { name = 'FL',  display = 'Front ~ Left' },
    { name = 'FDR', display = 'Front ~ Back-right' },
    { name = 'FDL', display = 'Front ~ Back-left' },
    { name = 'FBR', display = 'Front ~ Down-right' },
    { name = 'FBL', display = 'Front ~ Down-left' },
    { name = 'FPR', display = 'Front ~ Para-right' },
    { name = 'FPL', display = 'Front ~ Para-left' },
    { name = 'FPD', display = 'Front ~ Para-back' },
    { name = 'FPB', display = 'Front ~ Para-down' },
    { name = 'FX',  display = 'Front ~ X' },
    { name = 'FY',  display = 'Front ~ Y' },

    { name = 'BI',  display = 'Back ~ In' },
    { name = 'BF',  display = 'Back ~ Up' },
    { name = 'BU',  display = 'Back ~ Front' },
    { name = 'BR',  display = 'Back ~ Right' },
    { name = 'BL',  display = 'Back ~ Left' },
    { name = 'BDR', display = 'Back ~ Back-right' },
    { name = 'BDL', display = 'Back ~ Back-left' },
    { name = 'BBR', display = 'Back ~ Down-right' },
    { name = 'BBL', display = 'Back ~ Down-left' },
    { name = 'BPR', display = 'Back ~ Para-right' },
    { name = 'BPL', display = 'Back ~ Para-left' },
    { name = 'BPD', display = 'Back ~ Para-back' },
    { name = 'BPB', display = 'Back ~ Para-down' },
    { name = 'BX',  display = 'Back ~ X' },
    { name = 'BY',  display = 'Back ~ Y' },

    { name = 'OI',  display = 'Out ~ In' },
    { name = 'OF',  display = 'Out ~ Up' },
    { name = 'OU',  display = 'Out ~ Front' },
    { name = 'OR',  display = 'Out ~ Right' },
    { name = 'OL',  display = 'Out ~ Left' },
    { name = 'ODR', display = 'Out ~ Back-right' },
    { name = 'ODL', display = 'Out ~ Back-left' },
    { name = 'OBR', display = 'Out ~ Down-right' },
    { name = 'OBL', display = 'Out ~ Down-left' },
    { name = 'OPR', display = 'Out ~ Para-right' },
    { name = 'OPL', display = 'Out ~ Para-left' },
    { name = 'OPD', display = 'Out ~ Para-back' },
    { name = 'OPB', display = 'Out ~ Para-down' },
    { name = 'OX',  display = 'Out ~ X' },
    { name = 'OY',  display = 'Out ~ Y' },

    { name = 'II',  display = 'In ~ In' },
    { name = 'IF',  display = 'In ~ Up' },
    { name = 'IU',  display = 'In ~ Front' },
    { name = 'IR',  display = 'In ~ Right' },
    { name = 'IL',  display = 'In ~ Left' },
    { name = 'IDR', display = 'In ~ Back-right' },
    { name = 'IDL', display = 'In ~ Back-left' },
    { name = 'IBR', display = 'In ~ Down-right' },
    { name = 'IBL', display = 'In ~ Down-left' },
    { name = 'IPR', display = 'In ~ Para-right' },
    { name = 'IPL', display = 'In ~ Para-left' },
    { name = 'IPD', display = 'In ~ Para-back' },
    { name = 'IPB', display = 'In ~ Para-down' },
    { name = 'IX',  display = 'In ~ X' },
    { name = 'IY',  display = 'In ~ Y' },
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
