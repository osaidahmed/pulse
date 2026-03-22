module ProductionDataPipeline where

data LogLevel = Debug | Info | Warn | Error

data Event = Event
  { eventType :: String
  , eventPayload :: String
  , eventLevel :: LogLevel
  , eventTimestamp :: Int
  }

data PipelineConfig = PipelineConfig
  { batchSize :: Int
  , maxRetries :: Int
  , timeoutMs :: Int
  , outputFormat :: String
  }

processEvent :: PipelineConfig -> Event -> Either String String
processEvent cfg evt = case eventLevel evt of
  Error -> Left ("error event: " ++ eventPayload evt)
  Warn  -> if eventTimestamp evt > 0
             then Right (formatOutput cfg (eventPayload evt))
             else Left "invalid timestamp"
  Info  -> Right (formatOutput cfg (eventPayload evt))
  Debug -> if batchSize cfg > 0
             then Right (formatOutput cfg (eventPayload evt))
             else Left "debug disabled"

formatOutput :: PipelineConfig -> String -> String
formatOutput cfg payload = case outputFormat cfg of
  "json" -> "{\"data\":\"" ++ payload ++ "\"}"
  "csv"  -> payload
  "xml"  -> "<data>" ++ payload ++ "</data>"
  _      -> payload

validateConfig :: PipelineConfig -> [String]
validateConfig cfg = batchErr ++ retryErr ++ timeoutErr
  where
    batchErr
      | batchSize cfg <= 0   = ["batch size must be positive"]
      | batchSize cfg > 1000 = ["batch size too large"]
      | otherwise            = []
    retryErr
      | maxRetries cfg < 0 = ["retries cannot be negative"]
      | maxRetries cfg > 10 = ["too many retries"]
      | otherwise           = []
    timeoutErr
      | timeoutMs cfg <= 0    = ["timeout must be positive"]
      | timeoutMs cfg > 60000 = ["timeout too large"]
      | otherwise             = []

classifyEvents :: [Event] -> ([Event], [Event], [Event])
classifyEvents = foldr classify ([], [], [])
  where
    classify evt (errs, warns, infos) = case eventLevel evt of
      Error -> (evt : errs, warns, infos)
      Warn  -> (errs, evt : warns, infos)
      _     -> (errs, warns, evt : infos)

dispatchCommand :: String -> String
dispatchCommand cmd = case cmd of
  "start"    -> "starting"
  "stop"     -> "stopping"
  "restart"  -> "restarting"
  "status"   -> "checking"
  "deploy"   -> "deploying"
  "rollback" -> "rolling back"
  _          -> "unknown command"
