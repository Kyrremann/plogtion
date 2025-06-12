---
title: "{{ form.title }}"
date: "{{ form.date }}"
categories: "{{ form.categories }}"
main:
  image: "{{ form.main.image_url }}"
  alt_text: "{{ form.main.alt_text }}"
  caption: "{{ form.main.caption }}"
  location: "{{ form.main.location }}"
  coordinates: "{{ form.main.coordinates }}"
  coordinates_url: "https://www.google.com/maps/place/{{ form.main.coordinates }}"
strava: "{{ form.strava }}"
---

{{ form.main.description }}

{% for key, metadata in form.images -%}
{%- if loop.first %}{% continue %}{% endif -%}
![{{ metadata.alt_text }}]({{ metadata.image_url }})
*[{{ metadata.location }}](https://www.google.com/maps/place/{{ metadata.coordinates }}): {{ metadata.caption }}*

{{ metadata.description }}
{%- endfor -%}
