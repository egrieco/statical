use std::{iter, rc::Rc};

use super::{calendar_collection::CalendarCollection, event::Event};

type AgendaPageId = isize;
type EventSlice<'a> = Vec<Rc<Event>>;

/// A triple with the previous, current, and next agenda pages present
///
/// Note that the previous and next weeks may be None
pub type AgendaSlice<'a> = &'a [Option<(&'a AgendaPageId, &'a EventSlice<'a>)>];

pub(crate) struct Agenda {
    events: Vec<(Vec<Rc<Event>>, isize)>,
}

// We're splitting this into its own struct so we can search for the page in which a given event/date appears
impl Agenda {
    pub(crate) fn new(calendar_collection: &CalendarCollection) -> Self {
        // partition events into past and future events
        // TODO: might want to convert timezone on events before making the naive
        let (mut past_events, mut future_events): (Vec<_>, Vec<_>) = calendar_collection
            .events()
            .cloned()
            .partition(|e| e.start().date_naive() < calendar_collection.today_date());

        // process past events
        past_events.sort_by_key(|e| e.start());
        let mut past_events: Vec<_> = past_events
            .rchunks(calendar_collection.config.agenda_events_per_page)
            .map(|e| e.to_owned())
            .zip((1_isize..).map(|i| -i))
            .collect();
        past_events.reverse();

        // process future events
        future_events.sort_by_key(|e| e.start());
        let future_events_iter = future_events
            .chunks(calendar_collection.config.agenda_events_per_page)
            .map(|e| e.to_owned())
            .zip(0..);

        // combine all events into one list
        past_events.extend(future_events_iter);

        let events = past_events
            .into_iter()
            .collect::<Vec<(Vec<Rc<Event>>, AgendaPageId)>>();

        Agenda { events }
    }

    pub(crate) fn pages(&self) -> Vec<Option<(&isize, &EventSlice)>> {
        // chain a None to the list of agenda blocks and a None at the end
        // this will allow us to traverse the list as windows with the first and last
        // having None as appropriate
        let chained_iter = iter::once(None)
            .chain(
                self.events
                    .iter()
                    .map(|(events, page)| Some((page, events))),
            )
            .chain(iter::once(None));

        chained_iter.collect()
    }
}
