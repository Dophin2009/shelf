let Hook = ../Hook/Type.dhall

let File = ../FileProcess/Type

let LinkType = ../FileProcess/LinkType

let TemplateProcess = ../TemplateProcess/Type

let Package =
      { Type =
          { name : Text
          , dependencies : List Text
          , defaultLinkType : LinkType
          , ignorePatterns : List Text
          , files : List File.Type
          , templateFiles : List TemplateProcess.Type
          , beforeLink : List Hook
          , afterLink : List Hook
          , tree : Text
          }
      , default =
        { dependencies = [] : List Text
        , defaultLinkType = LinkType.Link
        , ignorePatterns = [] : List Text
        , files = [] : List File.Type
        , templateFiles = [] : List TemplateProcess.Type
        , beforeLink = [] : List Hook
        , afterLink = [] : List Hook
        , tree = Some "tree"
        }
      }

in  Package
