let Engine = ./Engine

let Template =
      { Type =
          { src : Text
          , dest : Text
          , engine : Engine
          , replaceFiles : Optional Bool
          , replaceDirectories : Optional Bool
          }
      , default =
        { engine = Engine.Gtmpl
        , replaceFiles = None Bool
        , replaceDirectories = None Bool
        }
      }

in  Template
