---
title: "{{ form.title }}"
date: {{ form.date }}
categories: {{ form.categories }}
image: {{ form.main_image }}
---

{{ form.description }}
{%- for key, metadata in form.images -%}
{%- if loop.first %}{% continue %}{% endif -%}
![{{ metadata.alt_text }}]({{ metadata.image }})
{{ metadata.description }}
{{ metadata.location }}
{%- endfor -%}
