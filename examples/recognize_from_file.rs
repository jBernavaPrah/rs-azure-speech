use std::{env, io, path};
use hound::{Sample, WavReader, WavWriter};
use log::{info, LevelFilter};
use simplelog::{ColorChoice, Config, TerminalMode, TermLogger};
use tokio::sync::mpsc::Sender;
use azure_speech::{Auth, recognizer};
use azure_speech::errors::Error;
use azure_speech::recognizer::{Details, Event, EventBase, Source};
use azure_speech::recognizer::config::{LanguageDetectMode, ResolverConfig};
use azure_speech::recognizer::speech::EventSpeech;

#[tokio::main]
async fn main() -> Result<(), Error> {
    
    // Initialize the logger
    TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Mixed, ColorChoice::Auto).unwrap();
    
    // Configure the resolver 
    let mut config = ResolverConfig::new(Auth::from_subscription(

        // Add your Azure region and subscription key here. 
        // Create a free account at https://azure.microsoft.com/en-us/try/cognitive-services/ to get the subscription key
        // and the region where the subscription is created.

        env::var("AZURE_REGION").expect("Region set on AZURE_REGION env"),
        env::var("AZURE_SUBSCRIPTION_KEY").expect("Subscription set on AZURE_SUBSCRIPTION_KEY env"),
    ));
    config.set_detect_languages(vec!["it-it", "en-us"], LanguageDetectMode::Continuous);
    //config.set_output_format(OutputFormat::Simple);
    // ...



    // Create a source for recognizer. This will be used to send the audio data to the recognizer
    let source = create_source_from_file("examples/audios/whatstheweatherlike.wav");
    let mut stream = recognizer::speech(config, source).await?;

    while let Some(r) = stream.recv().await {
        match r {
            // Base Events are associated with Event
            Event::Base(EventBase::Cancelled { reason}) => {
                info!("Cancelled {:?}", reason);
                break;
            }

            Event::Base(EventBase::SessionStarted { session_id }) => {
                info!("SessionStarted: {:?}", session_id);
            }

            Event::Base(EventBase::SessionStopped { session_id }) => {
                info!("SessionStopped: {:?}", session_id);
                break;
            }
            
            Event::Specific(EventSpeech::UnMatch { raw }) => {
                info!("UnMatch: {:?}", raw);
            }
            Event::Specific(EventSpeech::Recognized { text, raw, .. }) => {
                info!("Recognized: {} raw: {:?}", text, raw );
            }
            Event::Specific(EventSpeech::Recognizing { text, .. }) => {
                info!("Recognizing: {:?}", text);
            }

            _ => info!("Received: {:?}", r)
        }
    }

    info!("End of the recognition.");

    Ok(())
}

fn create_source_from_file<P: AsRef<path::Path>>(filename: P) -> Source {
    
    let file = WavReader::open(filename).expect("Error opening file");
    
    let (source_tx, source_rx) = tokio::sync::mpsc::channel(1024);
    
    let source = Source::new(source_rx, file.spec().into(), Details::file());

    // Read the audio data from the file and send it to the channel
    async fn read_from_file<S, R>(mut reader: WavReader<R>, sender: Sender<Vec<u8>>)
        where S: Sample + funty::Numeric + Send + 'static,
              R: io::Read,
              <S as funty::Numeric>::Bytes: AsRef<[u8]>, {
        while let Some(sample) = reader.samples::<S>().next() {
            let s = S::from(sample.expect("Error reading sample"));
            sender.send(s.to_le_bytes().as_ref().to_vec()).await.unwrap();
        }
    }

    // Spawn a task to read the file and push the data to the channel 
    let format = file.spec().sample_format;
    match format {
        hound::SampleFormat::Float => tokio::spawn(read_from_file::<f32, _>(file, source_tx)),
        hound::SampleFormat::Int => tokio::spawn(read_from_file::<i32, _>(file, source_tx)),
    };
    
    source
    
    
}


